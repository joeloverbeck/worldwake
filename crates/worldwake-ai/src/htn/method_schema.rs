use crate::planner_ops::PlannerOpKind;
use worldwake_core::{
    CommodityKind, EntityId, GoalKindDiscriminant, GoalPlanningBudget, MethodFailureKind,
    MethodSchemaId, MotiveSourceDiscriminant, Permille, Quantity, WorkstationTag,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodSchema {
    pub id: MethodSchemaId,
    pub goal_kind: GoalKindDiscriminant,
    pub preconditions: Vec<MethodPrecondition>,
    pub subgoals: Vec<SubgoalTemplate>,
    pub expected_artifacts: Vec<ArtifactTemplate>,
    pub required_claims: Vec<ClaimRequirement>,
    pub failure_modes: Vec<MethodFailureMode>,
    pub explanation_template: ExplanationTemplateId,
    pub motive_bias: Vec<MotiveBias>,
    pub planning_budget_hint: Option<GoalPlanningBudget>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MethodPrecondition {
    BeliefHolds(BeliefPredicate),
    MotiveSourcePresent(MotiveSourceDiscriminant),
    AgentRole(RoleTag),
    LocationKnown(EntityCriterion),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubgoalTemplate {
    AcquireCommodity {
        commodity: CommodityTemplate,
        min_quantity: Quantity,
    },
    TravelTo(LocationTemplate),
    ObserveTarget(EntityCriterion),
    AskWitness(TopicTemplate),
    InspectArtifact(ArtifactTemplate),
    PerformAction(PlannerOpKind, PayloadTemplate),
    ResolveCoordination(ClaimRequirement),
    ReturnTo(LocationTemplate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotiveBias {
    pub motive_variant: MotiveSourceDiscriminant,
    pub weight: Permille,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MethodFailureMode {
    PreconditionLost(BeliefPredicate),
    SubgoalUnachievable(usize),
    ArtifactNotProduced(ArtifactTemplate),
    ClaimDenied(ClaimRequirement),
    Timeout(u32),
}

impl From<&MethodFailureMode> for MethodFailureKind {
    fn from(mode: &MethodFailureMode) -> Self {
        match mode {
            MethodFailureMode::PreconditionLost(_) => Self::PreconditionLost,
            MethodFailureMode::SubgoalUnachievable(_) => Self::SubgoalUnachievable,
            MethodFailureMode::ArtifactNotProduced(_) => Self::ArtifactNotProduced,
            MethodFailureMode::ClaimDenied(_) => Self::ClaimDenied,
            MethodFailureMode::Timeout(_) => Self::Timeout,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BeliefPredicate {
    BountyRecordExists {
        bounty: EntityTemplate,
    },
    BountyExpired {
        bounty: EntityTemplate,
    },
    TargetLastSeenKnown {
        target: EntityTemplate,
    },
    WitnessNamesKnown {
        violation: EntityTemplate,
    },
    InstitutionalRecordBelievedExtant {
        violation: EntityTemplate,
    },
    ResourceSourceKnown {
        commodity: CommodityTemplate,
    },
    SellerKnown {
        commodity: CommodityTemplate,
    },
    OwnedCommodityBelowThreshold {
        commodity: CommodityTemplate,
        threshold: Quantity,
    },
    OwnsInputsForRecipe {
        recipe: RecipeTemplate,
    },
    EscorteeBelievedSafeAt {
        escortee: EntityTemplate,
    },
    AllyOrBountyOfficeAvailable,
    TargetBelievedDangerous {
        target: EntityTemplate,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityCriterion {
    Target(EntityTemplate),
    Workstation(WorkstationTag),
    ResourceSource(CommodityTemplate),
    Seller(CommodityTemplate),
    Witness { topic: TopicTemplate },
    ViolationEvidence { violation: EntityTemplate },
    Ledger { institution: EntityTemplate },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RoleTag {
    Hunter,
    Guard,
    Merchant,
    Magistrate,
    Crafter,
    Caravaneer,
    Civilian,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocationTemplate {
    LastKnownTargetPlace { target: EntityTemplate },
    NearestSellerOf { commodity: CommodityTemplate },
    AgentHome,
    BountyIssuerPlace { bounty: EntityTemplate },
    OfficePlace { institution: EntityTemplate },
    EscorteeHome { escortee: EntityTemplate },
    KnownWorkstationFor { recipe: RecipeTemplate },
    StagingPlaceForConfrontation { target: EntityTemplate },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TopicTemplate {
    TargetWhereabouts { target: EntityTemplate },
    ViolationCircumstances { violation: EntityTemplate },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayloadTemplate {
    FromContext,
    Explicit(PayloadValueTemplate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayloadValueTemplate {
    Trade {
        commodity: CommodityTemplate,
        quantity: Quantity,
    },
    Craft {
        recipe: RecipeTemplate,
    },
    Attack {
        target: EntityTemplate,
    },
    ClaimBounty {
        bounty: EntityTemplate,
    },
    EscortToSafety {
        escortee: EntityTemplate,
        destination: LocationTemplate,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtifactTemplate {
    ViolationEvidence {
        violation: EntityTemplate,
    },
    Ledger {
        institution: EntityTemplate,
    },
    BountyProof {
        bounty: EntityTemplate,
        target: EntityTemplate,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimRequirement {
    OfficeAuthority {
        office: EntityTemplate,
    },
    ResourceSourceAccess {
        commodity: CommodityTemplate,
        place: EntityTemplate,
    },
    BountyIssuance {
        bounty: EntityTemplate,
    },
    FacilityQueueSlot {
        facility: EntityTemplate,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityTemplate {
    GoalPrimaryEntity,
    GoalSecondaryEntity,
    GoalPlace,
    BountyTarget,
    Violation,
    Institution,
    Escortee,
    Fixed(EntityId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommodityTemplate {
    GoalCommodity,
    RecipeInput { recipe: RecipeTemplate, ordinal: u8 },
    Fixed(CommodityKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecipeTemplate {
    GoalRecipe,
    Fixed(u32),
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ExplanationTemplateId(pub u32);

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    #[test]
    fn method_failure_mode_to_kind_projection_covers_all_variants() {
        let target = entity(1);
        let place = entity(2);
        let institution = entity(3);

        let cases = [
            (
                MethodFailureMode::PreconditionLost(BeliefPredicate::TargetBelievedDangerous {
                    target: EntityTemplate::Fixed(target),
                }),
                MethodFailureKind::PreconditionLost,
            ),
            (
                MethodFailureMode::SubgoalUnachievable(1),
                MethodFailureKind::SubgoalUnachievable,
            ),
            (
                MethodFailureMode::ArtifactNotProduced(ArtifactTemplate::BountyProof {
                    bounty: EntityTemplate::Fixed(entity(4)),
                    target: EntityTemplate::Fixed(target),
                }),
                MethodFailureKind::ArtifactNotProduced,
            ),
            (
                MethodFailureMode::ClaimDenied(ClaimRequirement::ResourceSourceAccess {
                    commodity: CommodityTemplate::Fixed(CommodityKind::Grain),
                    place: EntityTemplate::Fixed(place),
                }),
                MethodFailureKind::ClaimDenied,
            ),
            (MethodFailureMode::Timeout(100), MethodFailureKind::Timeout),
        ];

        for (mode, expected) in cases {
            assert_eq!(MethodFailureKind::from(&mode), expected);
        }

        let _ = ArtifactTemplate::Ledger {
            institution: EntityTemplate::Fixed(institution),
        };
    }

    #[test]
    fn method_schema_constructs_and_clones() {
        let target = entity(10);
        let schema = MethodSchema {
            id: MethodSchemaId(7),
            goal_kind: GoalKindDiscriminant::ProduceCommodity,
            preconditions: vec![
                MethodPrecondition::BeliefHolds(BeliefPredicate::ResourceSourceKnown {
                    commodity: CommodityTemplate::Fixed(CommodityKind::Grain),
                }),
                MethodPrecondition::AgentRole(RoleTag::Crafter),
            ],
            subgoals: vec![
                SubgoalTemplate::AcquireCommodity {
                    commodity: CommodityTemplate::Fixed(CommodityKind::Grain),
                    min_quantity: Quantity(2),
                },
                SubgoalTemplate::PerformAction(
                    PlannerOpKind::Craft,
                    PayloadTemplate::Explicit(PayloadValueTemplate::Craft {
                        recipe: RecipeTemplate::Fixed(1),
                    }),
                ),
            ],
            expected_artifacts: vec![ArtifactTemplate::BountyProof {
                bounty: EntityTemplate::Fixed(entity(11)),
                target: EntityTemplate::Fixed(target),
            }],
            required_claims: vec![ClaimRequirement::FacilityQueueSlot {
                facility: EntityTemplate::Fixed(entity(12)),
            }],
            failure_modes: vec![MethodFailureMode::Timeout(50)],
            explanation_template: ExplanationTemplateId(3),
            motive_bias: vec![MotiveBias {
                motive_variant: MotiveSourceDiscriminant::Greed,
                weight: Permille::new_unchecked(250),
            }],
            planning_budget_hint: Some(GoalPlanningBudget::PRODUCTION),
        };

        let cloned = schema.clone();

        assert_eq!(cloned, schema);
        assert_eq!(cloned.subgoals.len(), 2);
    }
}
