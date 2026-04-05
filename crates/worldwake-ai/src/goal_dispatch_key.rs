use serde::{Deserialize, Serialize};
use worldwake_core::{GoalKind, PunishmentKind};

/// Payload-aware AI-internal dispatch identity derived from authoritative goal identity.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum GoalDispatchKey {
    ConsumeOwnedCommodity,
    AcquireSelfConsume,
    AcquireRecipeInput,
    AcquireRestock,
    Sleep,
    Relieve,
    Wash,
    EngageHostile,
    RaidTarget,
    ReduceDanger,
    RegroupWithFaction,
    EstablishBanditCamp,
    TreatWounds,
    ProduceCommodity,
    SellCommodity,
    RestockCommodity,
    MoveCargo,
    LootCorpse,
    BuryCorpse,
    FulfillBounty,
    ShareBelief,
    ClaimOffice,
    SupportCandidateForOffice,
    InvestigateViolation,
    Patrol,
    StealItem,
    Accuse,
    PunishFine,
    PunishExile,
}

impl GoalDispatchKey {
    pub const ALL: [Self; 29] = [
        Self::ConsumeOwnedCommodity,
        Self::AcquireSelfConsume,
        Self::AcquireRecipeInput,
        Self::AcquireRestock,
        Self::Sleep,
        Self::Relieve,
        Self::Wash,
        Self::EngageHostile,
        Self::RaidTarget,
        Self::ReduceDanger,
        Self::RegroupWithFaction,
        Self::EstablishBanditCamp,
        Self::TreatWounds,
        Self::ProduceCommodity,
        Self::SellCommodity,
        Self::RestockCommodity,
        Self::MoveCargo,
        Self::LootCorpse,
        Self::BuryCorpse,
        Self::FulfillBounty,
        Self::ShareBelief,
        Self::ClaimOffice,
        Self::SupportCandidateForOffice,
        Self::InvestigateViolation,
        Self::Patrol,
        Self::StealItem,
        Self::Accuse,
        Self::PunishFine,
        Self::PunishExile,
    ];

    #[must_use]
    pub const fn all() -> &'static [Self] {
        &Self::ALL
    }

    #[must_use]
    pub const fn from_goal_kind(goal: &GoalKind) -> Self {
        match goal {
            GoalKind::ConsumeOwnedCommodity { .. } => Self::ConsumeOwnedCommodity,
            GoalKind::AcquireCommodity { purpose, .. } => match purpose {
                worldwake_core::CommodityPurpose::SelfConsume => Self::AcquireSelfConsume,
                worldwake_core::CommodityPurpose::RecipeInput(_) => Self::AcquireRecipeInput,
                worldwake_core::CommodityPurpose::Restock => Self::AcquireRestock,
            },
            GoalKind::Sleep => Self::Sleep,
            GoalKind::Relieve => Self::Relieve,
            GoalKind::Wash => Self::Wash,
            GoalKind::EngageHostile { .. } => Self::EngageHostile,
            GoalKind::RaidTarget { .. } => Self::RaidTarget,
            GoalKind::ReduceDanger => Self::ReduceDanger,
            GoalKind::RegroupWithFaction { .. } => Self::RegroupWithFaction,
            GoalKind::EstablishBanditCamp { .. } => Self::EstablishBanditCamp,
            GoalKind::TreatWounds { .. } => Self::TreatWounds,
            GoalKind::ProduceCommodity { .. } => Self::ProduceCommodity,
            GoalKind::SellCommodity { .. } => Self::SellCommodity,
            GoalKind::RestockCommodity { .. } => Self::RestockCommodity,
            GoalKind::MoveCargo { .. } => Self::MoveCargo,
            GoalKind::LootCorpse { .. } => Self::LootCorpse,
            GoalKind::BuryCorpse { .. } => Self::BuryCorpse,
            GoalKind::FulfillBounty { .. } => Self::FulfillBounty,
            GoalKind::ShareBelief { .. } => Self::ShareBelief,
            GoalKind::ClaimOffice { .. } => Self::ClaimOffice,
            GoalKind::SupportCandidateForOffice { .. } => Self::SupportCandidateForOffice,
            GoalKind::InvestigateViolation { .. } => Self::InvestigateViolation,
            GoalKind::Patrol { .. } => Self::Patrol,
            GoalKind::StealItem { .. } => Self::StealItem,
            GoalKind::Accuse { .. } => Self::Accuse,
            GoalKind::PunishAccused { punishment, .. } => match punishment {
                PunishmentKind::Fine { .. } => Self::PunishFine,
                PunishmentKind::Exile { .. } => Self::PunishExile,
            },
        }
    }
}

impl From<&GoalKind> for GoalDispatchKey {
    fn from(goal: &GoalKind) -> Self {
        Self::from_goal_kind(goal)
    }
}

impl From<GoalKind> for GoalDispatchKey {
    fn from(goal: GoalKind) -> Self {
        Self::from(&goal)
    }
}

#[cfg(test)]
mod tests {
    use super::GoalDispatchKey;
    use worldwake_core::{
        CommodityKind, CommodityPurpose, EntityId, GoalKind, PunishmentKind, Quantity, RecipeId,
        RecordEntryId, TellTopic, ViolationId,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    #[test]
    fn test_goal_dispatch_key_payload_sensitive_acquire_splits() {
        let self_consume = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        };
        let recipe_input = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::RecipeInput(RecipeId(1)),
        };
        let restock = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::Restock,
        };

        assert_eq!(
            GoalDispatchKey::from(self_consume),
            GoalDispatchKey::AcquireSelfConsume
        );
        assert_eq!(
            GoalDispatchKey::from(recipe_input),
            GoalDispatchKey::AcquireRecipeInput
        );
        assert_eq!(
            GoalDispatchKey::from(restock),
            GoalDispatchKey::AcquireRestock
        );
    }

    #[test]
    fn test_goal_dispatch_key_payload_sensitive_punish_splits() {
        let fine = GoalKind::PunishAccused {
            office: entity(1),
            accused: entity(2),
            accusation_entry: RecordEntryId(3),
            punishment: PunishmentKind::Fine {
                commodity: CommodityKind::Coin,
                amount: Quantity(5),
            },
        };
        let exile = GoalKind::PunishAccused {
            office: entity(1),
            accused: entity(2),
            accusation_entry: RecordEntryId(3),
            punishment: PunishmentKind::Exile {
                from_faction: entity(4),
            },
        };

        assert_eq!(GoalDispatchKey::from(fine), GoalDispatchKey::PunishFine);
        assert_eq!(GoalDispatchKey::from(exile), GoalDispatchKey::PunishExile);
    }

    #[test]
    fn test_goal_dispatch_key_recipe_inputs_collapse_by_dispatch_shape() {
        let first = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Grain,
            purpose: CommodityPurpose::RecipeInput(RecipeId(1)),
        };
        let second = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::RecipeInput(RecipeId(99)),
        };

        assert_eq!(
            GoalDispatchKey::from(first),
            GoalDispatchKey::AcquireRecipeInput
        );
        assert_eq!(
            GoalDispatchKey::from(second),
            GoalDispatchKey::AcquireRecipeInput
        );
    }

    #[test]
    fn test_goal_dispatch_key_exhaustive_coverage() {
        let target = entity(2);
        let office = entity(3);
        let destination = entity(4);
        let crime_register = entity(5);
        let goals = [
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
            },
            GoalKind::Sleep,
            GoalKind::Relieve,
            GoalKind::Wash,
            GoalKind::EngageHostile { target },
            GoalKind::RaidTarget { target },
            GoalKind::ReduceDanger,
            GoalKind::RegroupWithFaction { faction: office },
            GoalKind::TreatWounds { patient: target },
            GoalKind::ProduceCommodity {
                recipe_id: RecipeId(7),
            },
            GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            },
            GoalKind::LootCorpse { corpse: target },
            GoalKind::BuryCorpse {
                corpse: target,
                burial_site: destination,
            },
            GoalKind::FulfillBounty { bounty: target },
            GoalKind::ShareBelief {
                listener: target,
                topic: TellTopic::EntityBelief { subject: office },
                communication_class: worldwake_core::CommunicationClass::Gossip,
            },
            GoalKind::ClaimOffice { office },
            GoalKind::SupportCandidateForOffice {
                office,
                candidate: target,
            },
            GoalKind::InvestigateViolation {
                violation_id: ViolationId(1),
                place: destination,
            },
            GoalKind::Patrol { place: destination },
            GoalKind::StealItem {
                target_item: target,
            },
            GoalKind::Accuse {
                crime_register,
                accused: target,
                violation_id: ViolationId(2),
            },
            GoalKind::PunishAccused {
                office,
                accused: target,
                accusation_entry: RecordEntryId(3),
                punishment: PunishmentKind::Fine {
                    commodity: CommodityKind::Coin,
                    amount: Quantity(5),
                },
            },
        ];

        assert_eq!(goals.len(), 25);
        for goal in goals {
            let _ = GoalDispatchKey::from(goal);
        }
    }

    #[test]
    fn test_goal_dispatch_key_maps_patrol_goal() {
        let patrol = GoalKind::Patrol { place: entity(42) };

        assert_eq!(GoalDispatchKey::from(patrol), GoalDispatchKey::Patrol);
    }

    #[test]
    fn test_goal_dispatch_key_all_lists_each_dispatch_key_once() {
        assert_eq!(GoalDispatchKey::all(), &GoalDispatchKey::ALL);
        assert_eq!(GoalDispatchKey::all().len(), 29);
        for (idx, key) in GoalDispatchKey::all().iter().enumerate() {
            assert!(
                !GoalDispatchKey::all()[idx + 1..].contains(key),
                "duplicate key in exhaustive dispatch-key list: {key:?}"
            );
        }
    }

    #[test]
    fn test_goal_dispatch_key_assigns_distinct_bandit_goal_variants() {
        let target = entity(40);
        let faction = entity(41);

        assert_eq!(
            GoalDispatchKey::from(GoalKind::RaidTarget { target }),
            GoalDispatchKey::RaidTarget
        );
        assert_eq!(
            GoalDispatchKey::from(GoalKind::RegroupWithFaction { faction }),
            GoalDispatchKey::RegroupWithFaction
        );
        assert_ne!(GoalDispatchKey::RaidTarget, GoalDispatchKey::EngageHostile);
        assert_ne!(
            GoalDispatchKey::RegroupWithFaction,
            GoalDispatchKey::ReduceDanger
        );
    }
}
