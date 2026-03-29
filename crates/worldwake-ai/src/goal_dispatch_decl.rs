use crate::{goal_dispatch_key::GoalDispatchKey, PlannerOpKind, RankedGoalProvenanceFamily};
use worldwake_core::HomeostaticNeedId;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum InvalidationStrategy {
    CommodityOnly,
    AcquireCommodity,
    AcquireRestock,
    NeedWithFacilities(HomeostaticNeedId),
    NeedWithPosition(HomeostaticNeedId),
    CombatTarget,
    DangerReduction,
    TreatWounds,
    ProduceCommodity,
    PositionAndCommodity,
    PositionCommodityAndCoin,
    PositionAndTargetDead,
    ClaimOffice,
    SupportCandidateForOffice,
    InvestigateViolation,
    PunishAccused,
}

pub struct GoalDispatchDeclaration {
    pub trace_label: &'static str,
    pub provenance_family: Option<RankedGoalProvenanceFamily>,
    pub relevant_ops: &'static [PlannerOpKind],
    pub invalidation_strategy: InvalidationStrategy,
}

const CONSUME_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Consume,
    PlannerOpKind::Travel,
    PlannerOpKind::MoveCargo,
];
const ACQUIRE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Harvest,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const SLEEP_OPS: &[PlannerOpKind] = &[PlannerOpKind::Sleep, PlannerOpKind::Travel];
const RELIEVE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Relieve, PlannerOpKind::Travel];
const WASH_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Wash,
    PlannerOpKind::Travel,
    PlannerOpKind::MoveCargo,
];
const ENGAGE_HOSTILE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Attack];
const REDUCE_DANGER_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Defend,
    PlannerOpKind::Heal,
];
const TREAT_WOUNDS_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Heal,
    PlannerOpKind::Trade,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
    PlannerOpKind::Harvest,
];
const PRODUCE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const SELL_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::MoveCargo,
];
const RESTOCK_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Harvest,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const MOVE_CARGO_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::MoveCargo];
const LOOT_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Loot];
const BURY_OPS: &[PlannerOpKind] = &[PlannerOpKind::Bury];
const SHARE_BELIEF_OPS: &[PlannerOpKind] = &[PlannerOpKind::Tell];
const CLAIM_OFFICE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::ConsultRecord,
    PlannerOpKind::Bribe,
    PlannerOpKind::Threaten,
    PlannerOpKind::DeclareSupport,
    PlannerOpKind::PressForceClaim,
];
const SUPPORT_OFFICE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::ConsultRecord,
    PlannerOpKind::DeclareSupport,
];
const INVESTIGATE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Investigate];
const ACCUSE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Accuse];
const FINE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Fine];
const EXILE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Exile];

static DECL_CONSUME_OWNED_COMMODITY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ConsumeOwnedCommodity",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: CONSUME_OPS,
    invalidation_strategy: InvalidationStrategy::CommodityOnly,
};
static DECL_ACQUIRE_SELF_CONSUME: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "AcquireCommodity(SelfConsume)",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: ACQUIRE_OPS,
    invalidation_strategy: InvalidationStrategy::AcquireCommodity,
};
static DECL_ACQUIRE_RECIPE_INPUT: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "AcquireCommodity(RecipeInput)",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: ACQUIRE_OPS,
    invalidation_strategy: InvalidationStrategy::AcquireCommodity,
};
static DECL_ACQUIRE_RESTOCK: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "AcquireCommodity(Restock)",
    provenance_family: None,
    relevant_ops: ACQUIRE_OPS,
    invalidation_strategy: InvalidationStrategy::AcquireRestock,
};
static DECL_SLEEP: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "Sleep",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: SLEEP_OPS,
    invalidation_strategy: InvalidationStrategy::NeedWithFacilities(HomeostaticNeedId::Fatigue),
};
static DECL_RELIEVE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "Relieve",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: RELIEVE_OPS,
    invalidation_strategy: InvalidationStrategy::NeedWithPosition(HomeostaticNeedId::Bladder),
};
static DECL_WASH: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "Wash",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: WASH_OPS,
    invalidation_strategy: InvalidationStrategy::NeedWithFacilities(HomeostaticNeedId::Dirtiness),
};
static DECL_ENGAGE_HOSTILE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "EngageHostile",
    provenance_family: Some(RankedGoalProvenanceFamily::Danger),
    relevant_ops: ENGAGE_HOSTILE_OPS,
    invalidation_strategy: InvalidationStrategy::CombatTarget,
};
static DECL_REDUCE_DANGER: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ReduceDanger",
    provenance_family: Some(RankedGoalProvenanceFamily::Danger),
    relevant_ops: REDUCE_DANGER_OPS,
    invalidation_strategy: InvalidationStrategy::DangerReduction,
};
static DECL_TREAT_WOUNDS: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "TreatWounds",
    provenance_family: None,
    relevant_ops: TREAT_WOUNDS_OPS,
    invalidation_strategy: InvalidationStrategy::TreatWounds,
};
static DECL_PRODUCE_COMMODITY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ProduceCommodity",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: PRODUCE_OPS,
    invalidation_strategy: InvalidationStrategy::ProduceCommodity,
};
static DECL_SELL_COMMODITY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "SellCommodity",
    provenance_family: None,
    relevant_ops: SELL_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndCommodity,
};
static DECL_RESTOCK_COMMODITY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "RestockCommodity",
    provenance_family: None,
    relevant_ops: RESTOCK_OPS,
    invalidation_strategy: InvalidationStrategy::PositionCommodityAndCoin,
};
static DECL_MOVE_CARGO: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "MoveCargo",
    provenance_family: None,
    relevant_ops: MOVE_CARGO_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndCommodity,
};
static DECL_LOOT_CORPSE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "LootCorpse",
    provenance_family: None,
    relevant_ops: LOOT_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
};
static DECL_BURY_CORPSE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "BuryCorpse",
    provenance_family: None,
    relevant_ops: BURY_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
};
static DECL_SHARE_BELIEF: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ShareBelief",
    provenance_family: None,
    relevant_ops: SHARE_BELIEF_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
};
static DECL_CLAIM_OFFICE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ClaimOffice",
    provenance_family: None,
    relevant_ops: CLAIM_OFFICE_OPS,
    invalidation_strategy: InvalidationStrategy::ClaimOffice,
};
static DECL_SUPPORT_CANDIDATE_FOR_OFFICE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "SupportCandidateForOffice",
    provenance_family: None,
    relevant_ops: SUPPORT_OFFICE_OPS,
    invalidation_strategy: InvalidationStrategy::SupportCandidateForOffice,
};
static DECL_INVESTIGATE_VIOLATION: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "InvestigateViolation",
    provenance_family: None,
    relevant_ops: INVESTIGATE_OPS,
    invalidation_strategy: InvalidationStrategy::InvestigateViolation,
};
static DECL_STEAL_ITEM: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "StealItem",
    provenance_family: None,
    relevant_ops: MOVE_CARGO_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
};
static DECL_ACCUSE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "Accuse",
    provenance_family: None,
    relevant_ops: ACCUSE_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
};
static DECL_PUNISH_FINE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "PunishAccused(Fine)",
    provenance_family: None,
    relevant_ops: FINE_OPS,
    invalidation_strategy: InvalidationStrategy::PunishAccused,
};
static DECL_PUNISH_EXILE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "PunishAccused(Exile)",
    provenance_family: None,
    relevant_ops: EXILE_OPS,
    invalidation_strategy: InvalidationStrategy::PunishAccused,
};

impl GoalDispatchKey {
    #[must_use]
    pub const fn declaration(&self) -> &'static GoalDispatchDeclaration {
        match self {
            Self::ConsumeOwnedCommodity => &DECL_CONSUME_OWNED_COMMODITY,
            Self::AcquireSelfConsume => &DECL_ACQUIRE_SELF_CONSUME,
            Self::AcquireRecipeInput => &DECL_ACQUIRE_RECIPE_INPUT,
            Self::AcquireRestock => &DECL_ACQUIRE_RESTOCK,
            Self::Sleep => &DECL_SLEEP,
            Self::Relieve => &DECL_RELIEVE,
            Self::Wash => &DECL_WASH,
            Self::EngageHostile => &DECL_ENGAGE_HOSTILE,
            Self::ReduceDanger => &DECL_REDUCE_DANGER,
            Self::TreatWounds => &DECL_TREAT_WOUNDS,
            Self::ProduceCommodity => &DECL_PRODUCE_COMMODITY,
            Self::SellCommodity => &DECL_SELL_COMMODITY,
            Self::RestockCommodity => &DECL_RESTOCK_COMMODITY,
            Self::MoveCargo => &DECL_MOVE_CARGO,
            Self::LootCorpse => &DECL_LOOT_CORPSE,
            Self::BuryCorpse => &DECL_BURY_CORPSE,
            Self::ShareBelief => &DECL_SHARE_BELIEF,
            Self::ClaimOffice => &DECL_CLAIM_OFFICE,
            Self::SupportCandidateForOffice => &DECL_SUPPORT_CANDIDATE_FOR_OFFICE,
            Self::InvestigateViolation => &DECL_INVESTIGATE_VIOLATION,
            Self::StealItem => &DECL_STEAL_ITEM,
            Self::Accuse => &DECL_ACCUSE,
            Self::PunishFine => &DECL_PUNISH_FINE,
            Self::PunishExile => &DECL_PUNISH_EXILE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GoalDispatchDeclaration, InvalidationStrategy};
    use crate::{GoalDispatchKey, GoalKindPlannerExt};
    use worldwake_core::{
        CommodityKind, CommodityPurpose, EntityId, GoalKind, PunishmentKind, Quantity, RecipeId,
        RecordEntryId, TellTopic, ViolationId, HomeostaticNeedId,
    };

    const ALL_KEYS: &[GoalDispatchKey] = &[
        GoalDispatchKey::ConsumeOwnedCommodity,
        GoalDispatchKey::AcquireSelfConsume,
        GoalDispatchKey::AcquireRecipeInput,
        GoalDispatchKey::AcquireRestock,
        GoalDispatchKey::Sleep,
        GoalDispatchKey::Relieve,
        GoalDispatchKey::Wash,
        GoalDispatchKey::EngageHostile,
        GoalDispatchKey::ReduceDanger,
        GoalDispatchKey::TreatWounds,
        GoalDispatchKey::ProduceCommodity,
        GoalDispatchKey::SellCommodity,
        GoalDispatchKey::RestockCommodity,
        GoalDispatchKey::MoveCargo,
        GoalDispatchKey::LootCorpse,
        GoalDispatchKey::BuryCorpse,
        GoalDispatchKey::ShareBelief,
        GoalDispatchKey::ClaimOffice,
        GoalDispatchKey::SupportCandidateForOffice,
        GoalDispatchKey::InvestigateViolation,
        GoalDispatchKey::StealItem,
        GoalDispatchKey::Accuse,
        GoalDispatchKey::PunishFine,
        GoalDispatchKey::PunishExile,
    ];

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn representative_goal_for(key: GoalDispatchKey) -> GoalKind {
        let target = entity(2);
        let office = entity(3);
        let destination = entity(4);

        match key {
            GoalDispatchKey::ConsumeOwnedCommodity => GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalDispatchKey::AcquireSelfConsume => GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
            },
            GoalDispatchKey::AcquireRecipeInput => GoalKind::AcquireCommodity {
                commodity: CommodityKind::Grain,
                purpose: CommodityPurpose::RecipeInput(RecipeId(7)),
            },
            GoalDispatchKey::AcquireRestock => GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::Restock,
            },
            GoalDispatchKey::Sleep => GoalKind::Sleep,
            GoalDispatchKey::Relieve => GoalKind::Relieve,
            GoalDispatchKey::Wash => GoalKind::Wash,
            GoalDispatchKey::EngageHostile => GoalKind::EngageHostile { target },
            GoalDispatchKey::ReduceDanger => GoalKind::ReduceDanger,
            GoalDispatchKey::TreatWounds => GoalKind::TreatWounds { patient: target },
            GoalDispatchKey::ProduceCommodity => GoalKind::ProduceCommodity {
                recipe_id: RecipeId(11),
            },
            GoalDispatchKey::SellCommodity => GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalDispatchKey::RestockCommodity => GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalDispatchKey::MoveCargo => GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            },
            GoalDispatchKey::LootCorpse => GoalKind::LootCorpse { corpse: target },
            GoalDispatchKey::BuryCorpse => GoalKind::BuryCorpse {
                corpse: target,
                burial_site: destination,
            },
            GoalDispatchKey::ShareBelief => GoalKind::ShareBelief {
                listener: target,
                topic: TellTopic::EntityBelief { subject: office },
            },
            GoalDispatchKey::ClaimOffice => GoalKind::ClaimOffice { office },
            GoalDispatchKey::SupportCandidateForOffice => GoalKind::SupportCandidateForOffice {
                office,
                candidate: target,
            },
            GoalDispatchKey::InvestigateViolation => GoalKind::InvestigateViolation {
                violation_id: ViolationId(1),
                place: destination,
            },
            GoalDispatchKey::StealItem => GoalKind::StealItem {
                target_item: target,
            },
            GoalDispatchKey::Accuse => GoalKind::Accuse {
                crime_register: office,
                accused: target,
                violation_id: ViolationId(2),
            },
            GoalDispatchKey::PunishFine => GoalKind::PunishAccused {
                office,
                accused: target,
                accusation_entry: RecordEntryId(3),
                punishment: PunishmentKind::Fine {
                    commodity: CommodityKind::Coin,
                    amount: Quantity(5),
                },
            },
            GoalDispatchKey::PunishExile => GoalKind::PunishAccused {
                office,
                accused: target,
                accusation_entry: RecordEntryId(3),
                punishment: PunishmentKind::Exile {
                    from_faction: destination,
                },
            },
        }
    }

    #[test]
    fn test_declaration_completeness() {
        assert_eq!(ALL_KEYS.len(), 24);

        for key in ALL_KEYS {
            let declaration: &'static GoalDispatchDeclaration = key.declaration();
            assert!(!declaration.trace_label.is_empty());
        }
    }

    #[test]
    fn test_declaration_provenance_matches_live_goal_model() {
        for key in ALL_KEYS {
            let goal = representative_goal_for(*key);
            assert_eq!(
                key.declaration().provenance_family,
                goal.ranked_goal_provenance_family(),
                "provenance mismatch for {key:?}"
            );
        }
    }

    #[test]
    fn test_declaration_relevant_ops_match_live_goal_model() {
        for key in ALL_KEYS {
            let goal = representative_goal_for(*key);
            assert_eq!(
                key.declaration().relevant_ops,
                goal.relevant_op_kinds(),
                "relevant_ops mismatch for {key:?}"
            );
        }
    }

    #[test]
    fn test_punish_fine_vs_exile_ops() {
        assert_ne!(
            GoalDispatchKey::PunishFine.declaration().relevant_ops,
            GoalDispatchKey::PunishExile.declaration().relevant_ops
        );
    }

    #[test]
    fn test_trace_labels_nonempty_and_distinct_for_payload_splits() {
        for key in ALL_KEYS {
            assert!(!key.declaration().trace_label.is_empty(), "{key:?}");
        }

        assert_ne!(
            GoalDispatchKey::AcquireSelfConsume.declaration().trace_label,
            GoalDispatchKey::AcquireRecipeInput.declaration().trace_label
        );
        assert_ne!(
            GoalDispatchKey::AcquireRecipeInput.declaration().trace_label,
            GoalDispatchKey::AcquireRestock.declaration().trace_label
        );
        assert_ne!(
            GoalDispatchKey::PunishFine.declaration().trace_label,
            GoalDispatchKey::PunishExile.declaration().trace_label
        );
    }

    #[test]
    fn test_invalidation_strategies_cover_all_declarations() {
        for key in ALL_KEYS {
            let declaration = key.declaration();
            match declaration.invalidation_strategy {
                InvalidationStrategy::CommodityOnly
                | InvalidationStrategy::AcquireCommodity
                | InvalidationStrategy::AcquireRestock
                | InvalidationStrategy::CombatTarget
                | InvalidationStrategy::DangerReduction
                | InvalidationStrategy::TreatWounds
                | InvalidationStrategy::ProduceCommodity
                | InvalidationStrategy::PositionAndCommodity
                | InvalidationStrategy::PositionCommodityAndCoin
                | InvalidationStrategy::PositionAndTargetDead
                | InvalidationStrategy::ClaimOffice
                | InvalidationStrategy::SupportCandidateForOffice
                | InvalidationStrategy::InvestigateViolation
                | InvalidationStrategy::PunishAccused => {}
                InvalidationStrategy::NeedWithFacilities(need)
                | InvalidationStrategy::NeedWithPosition(need) => {
                    assert!(
                        matches!(
                            need,
                            HomeostaticNeedId::Fatigue
                                | HomeostaticNeedId::Bladder
                                | HomeostaticNeedId::Dirtiness
                        ),
                        "{key:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_invalidation_strategies_match_payload_sensitive_and_shared_families() {
        assert_eq!(
            GoalDispatchKey::AcquireSelfConsume
                .declaration()
                .invalidation_strategy,
            InvalidationStrategy::AcquireCommodity
        );
        assert_eq!(
            GoalDispatchKey::AcquireRecipeInput
                .declaration()
                .invalidation_strategy,
            InvalidationStrategy::AcquireCommodity
        );
        assert_eq!(
            GoalDispatchKey::AcquireRestock
                .declaration()
                .invalidation_strategy,
            InvalidationStrategy::AcquireRestock
        );
        assert_eq!(
            GoalDispatchKey::LootCorpse.declaration().invalidation_strategy,
            GoalDispatchKey::BuryCorpse.declaration().invalidation_strategy
        );
        assert_eq!(
            GoalDispatchKey::SellCommodity
                .declaration()
                .invalidation_strategy,
            GoalDispatchKey::MoveCargo.declaration().invalidation_strategy
        );
        assert_eq!(
            GoalDispatchKey::PunishFine.declaration().invalidation_strategy,
            GoalDispatchKey::PunishExile.declaration().invalidation_strategy
        );
    }
}
