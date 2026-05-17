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
        commodity: CommodityKind,
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
        bounty: EntityId,
    },
    BountyExpired {
        bounty: EntityId,
    },
    TargetLastSeenKnown {
        target: EntityId,
    },
    WitnessNamesKnown {
        violation: EntityId,
    },
    InstitutionalRecordBelievedExtant {
        violation: EntityId,
    },
    ResourceSourceKnown {
        commodity: CommodityKind,
    },
    SellerKnown {
        commodity: CommodityKind,
    },
    OwnedCommodityBelowThreshold {
        commodity: CommodityKind,
        threshold: Quantity,
    },
    OwnsInputsForRecipe {
        recipe_id: u32,
    },
    EscorteeBelievedSafeAt {
        escortee: EntityId,
    },
    AllyOrBountyOfficeAvailable,
    TargetBelievedDangerous {
        target: EntityId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityCriterion {
    Target(EntityId),
    Workstation(WorkstationTag),
    ResourceSource(CommodityKind),
    Seller(CommodityKind),
    Witness { topic: TopicTemplate },
    ViolationEvidence { violation: EntityId },
    Ledger { institution: EntityId },
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
    LastKnownTargetPlace { target: EntityId },
    NearestSellerOf { commodity: CommodityKind },
    AgentHome,
    BountyIssuerPlace { bounty: EntityId },
    OfficePlace { institution: EntityId },
    EscorteeHome { escortee: EntityId },
    KnownWorkstationFor { recipe_id: u32 },
    StagingPlaceForConfrontation { target: EntityId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TopicTemplate {
    TargetWhereabouts { target: EntityId },
    ViolationCircumstances { violation: EntityId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayloadTemplate {
    FromContext,
    Explicit(PayloadValueTemplate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayloadValueTemplate {
    Trade {
        commodity: CommodityKind,
        quantity: Quantity,
    },
    Craft {
        recipe_id: u32,
    },
    Attack {
        target: EntityId,
    },
    ClaimBounty {
        bounty: EntityId,
    },
    EscortToSafety {
        escortee: EntityId,
        destination: LocationTemplate,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtifactTemplate {
    ViolationEvidence { violation: EntityId },
    Ledger { institution: EntityId },
    BountyProof { bounty: EntityId, target: EntityId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimRequirement {
    OfficeAuthority {
        office: EntityId,
    },
    ResourceSourceAccess {
        commodity: CommodityKind,
        place: EntityId,
    },
    BountyIssuance {
        bounty: EntityId,
    },
    FacilityQueueSlot {
        facility: EntityId,
    },
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
                    target,
                }),
                MethodFailureKind::PreconditionLost,
            ),
            (
                MethodFailureMode::SubgoalUnachievable(1),
                MethodFailureKind::SubgoalUnachievable,
            ),
            (
                MethodFailureMode::ArtifactNotProduced(ArtifactTemplate::BountyProof {
                    bounty: entity(4),
                    target,
                }),
                MethodFailureKind::ArtifactNotProduced,
            ),
            (
                MethodFailureMode::ClaimDenied(ClaimRequirement::ResourceSourceAccess {
                    commodity: CommodityKind::Grain,
                    place,
                }),
                MethodFailureKind::ClaimDenied,
            ),
            (MethodFailureMode::Timeout(100), MethodFailureKind::Timeout),
        ];

        for (mode, expected) in cases {
            assert_eq!(MethodFailureKind::from(&mode), expected);
        }

        let _ = ArtifactTemplate::Ledger { institution };
    }

    #[test]
    fn method_schema_constructs_and_clones() {
        let target = entity(10);
        let schema = MethodSchema {
            id: MethodSchemaId(7),
            goal_kind: GoalKindDiscriminant::ProduceCommodity,
            preconditions: vec![
                MethodPrecondition::BeliefHolds(BeliefPredicate::ResourceSourceKnown {
                    commodity: CommodityKind::Grain,
                }),
                MethodPrecondition::AgentRole(RoleTag::Crafter),
            ],
            subgoals: vec![
                SubgoalTemplate::AcquireCommodity {
                    commodity: CommodityKind::Grain,
                    min_quantity: Quantity(2),
                },
                SubgoalTemplate::PerformAction(
                    PlannerOpKind::Craft,
                    PayloadTemplate::Explicit(PayloadValueTemplate::Craft { recipe_id: 1 }),
                ),
            ],
            expected_artifacts: vec![ArtifactTemplate::BountyProof {
                bounty: entity(11),
                target,
            }],
            required_claims: vec![ClaimRequirement::FacilityQueueSlot {
                facility: entity(12),
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
