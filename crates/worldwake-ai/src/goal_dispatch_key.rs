use serde::{Deserialize, Serialize};
use worldwake_core::{GoalKind, NoticeTopic, PunishmentKind};

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
    FreeCarryCapacity,
    EngageHostile,
    RaidTarget,
    ReduceDanger,
    RegroupWithFaction,
    EstablishBanditCamp,
    TreatWounds,
    SearchForMissing,
    ReportMissing,
    ReportFound,
    EscortToSafety,
    ProduceCommodity,
    SellCommodity,
    RestockCommodity,
    MoveCargo,
    LootCorpse,
    BuryCorpse,
    FulfillBounty,
    PostBounty,
    PostNoticeWarning,
    PostNoticeOther,
    ShareBeliefAlarm,
    ShareBeliefTestimony,
    ShareBeliefGossip,
    AskWitness,
    ClaimOffice,
    SupportCandidateForOffice,
    InvestigateViolation,
    Patrol,
    ExploreLocation,
    StealItem,
    Accuse,
    PunishFine,
    PunishExile,
}

impl GoalDispatchKey {
    pub const ALL: [Self; 41] = [
        Self::ConsumeOwnedCommodity,
        Self::AcquireSelfConsume,
        Self::AcquireRecipeInput,
        Self::AcquireRestock,
        Self::Sleep,
        Self::Relieve,
        Self::Wash,
        Self::FreeCarryCapacity,
        Self::EngageHostile,
        Self::RaidTarget,
        Self::ReduceDanger,
        Self::RegroupWithFaction,
        Self::EstablishBanditCamp,
        Self::TreatWounds,
        Self::SearchForMissing,
        Self::ReportMissing,
        Self::ReportFound,
        Self::EscortToSafety,
        Self::ProduceCommodity,
        Self::SellCommodity,
        Self::RestockCommodity,
        Self::MoveCargo,
        Self::LootCorpse,
        Self::BuryCorpse,
        Self::FulfillBounty,
        Self::PostBounty,
        Self::PostNoticeWarning,
        Self::PostNoticeOther,
        Self::ShareBeliefAlarm,
        Self::ShareBeliefTestimony,
        Self::ShareBeliefGossip,
        Self::AskWitness,
        Self::ClaimOffice,
        Self::SupportCandidateForOffice,
        Self::InvestigateViolation,
        Self::Patrol,
        Self::ExploreLocation,
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
            GoalKind::FreeCarryCapacity => Self::FreeCarryCapacity,
            GoalKind::EngageHostile { .. } => Self::EngageHostile,
            GoalKind::RaidTarget { .. } => Self::RaidTarget,
            GoalKind::ReduceDanger => Self::ReduceDanger,
            GoalKind::RegroupWithFaction { .. } => Self::RegroupWithFaction,
            GoalKind::EstablishBanditCamp { .. } => Self::EstablishBanditCamp,
            GoalKind::TreatWounds { .. } => Self::TreatWounds,
            GoalKind::SearchForMissing { .. } => Self::SearchForMissing,
            GoalKind::ReportMissing { .. } => Self::ReportMissing,
            GoalKind::ReportFound { .. } => Self::ReportFound,
            GoalKind::EscortToSafety { .. } => Self::EscortToSafety,
            GoalKind::ProduceCommodity { .. } => Self::ProduceCommodity,
            GoalKind::SellCommodity { .. } => Self::SellCommodity,
            GoalKind::RestockCommodity { .. } => Self::RestockCommodity,
            GoalKind::MoveCargo { .. } => Self::MoveCargo,
            GoalKind::LootCorpse { .. } => Self::LootCorpse,
            GoalKind::BuryCorpse { .. } => Self::BuryCorpse,
            GoalKind::FulfillBounty { .. } => Self::FulfillBounty,
            GoalKind::PostBounty { .. } => Self::PostBounty,
            GoalKind::PostNotice { topic, .. } => match topic {
                NoticeTopic::ThreatWarning { .. } => Self::PostNoticeWarning,
                _ => Self::PostNoticeOther,
            },
            GoalKind::ShareBelief {
                communication_class,
                ..
            } => match communication_class {
                worldwake_core::CommunicationClass::Alarm => Self::ShareBeliefAlarm,
                worldwake_core::CommunicationClass::Testimony => Self::ShareBeliefTestimony,
                worldwake_core::CommunicationClass::Gossip => Self::ShareBeliefGossip,
            },
            GoalKind::AskWitness { .. } => Self::AskWitness,
            GoalKind::ClaimOffice { .. } => Self::ClaimOffice,
            GoalKind::SupportCandidateForOffice { .. } => Self::SupportCandidateForOffice,
            GoalKind::InvestigateViolation { .. } => Self::InvestigateViolation,
            GoalKind::Patrol { .. } => Self::Patrol,
            GoalKind::ExploreLocation { .. } => Self::ExploreLocation,
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
        AcquisitionQuantity, ArtifactPostingContext, CommodityKind, CommodityPurpose,
        CommunicationClass, EntityId, ExpectationId, GoalKind, InstitutionalClaim, NoticeTopic,
        PunishmentKind, Quantity, RecipeId, RecordEntryId, TellTopic, ViolationId,
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
            quantity: AcquisitionQuantity::single(),
        };
        let recipe_input = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::RecipeInput(RecipeId(1)),
            quantity: AcquisitionQuantity::single(),
        };
        let restock = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::Restock,
            quantity: AcquisitionQuantity::single(),
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
    fn test_goal_dispatch_key_payload_sensitive_share_belief_splits() {
        let alarm = GoalKind::ShareBelief {
            listener: entity(1),
            topic: TellTopic::EntityBelief { subject: entity(2) },
            communication_class: CommunicationClass::Alarm,
        };
        let testimony = GoalKind::ShareBelief {
            listener: entity(1),
            topic: TellTopic::EntityBelief { subject: entity(2) },
            communication_class: CommunicationClass::Testimony,
        };
        let gossip = GoalKind::ShareBelief {
            listener: entity(1),
            topic: TellTopic::EntityBelief { subject: entity(2) },
            communication_class: CommunicationClass::Gossip,
        };

        assert_eq!(
            GoalDispatchKey::from(alarm),
            GoalDispatchKey::ShareBeliefAlarm
        );
        assert_eq!(
            GoalDispatchKey::from(testimony),
            GoalDispatchKey::ShareBeliefTestimony
        );
        assert_eq!(
            GoalDispatchKey::from(gossip),
            GoalDispatchKey::ShareBeliefGossip
        );
    }

    #[test]
    fn test_goal_dispatch_key_payload_sensitive_post_notice_splits() {
        let posting = ArtifactPostingContext {
            posting_place: entity(10),
            issuing_authority: None,
            expires_at: None,
            jurisdiction: None,
        };
        let warning = GoalKind::PostNotice {
            posting,
            topic: NoticeTopic::ThreatWarning { place: entity(11) },
        };
        let vacancy = GoalKind::PostNotice {
            posting,
            topic: NoticeTopic::OfficeVacancy { office: entity(12) },
        };
        let shortage = GoalKind::PostNotice {
            posting,
            topic: NoticeTopic::CommodityShortage {
                commodity: CommodityKind::Bread,
                place: entity(13),
            },
        };
        let institutional = GoalKind::PostNotice {
            posting,
            topic: NoticeTopic::Institutional {
                claim: InstitutionalClaim::OfficeHolder {
                    office: entity(14),
                    holder: Some(entity(15)),
                    effective_tick: worldwake_core::Tick(0),
                },
            },
        };

        assert_eq!(
            GoalDispatchKey::from(warning),
            GoalDispatchKey::PostNoticeWarning
        );
        assert_eq!(
            GoalDispatchKey::from(vacancy),
            GoalDispatchKey::PostNoticeOther
        );
        assert_eq!(
            GoalDispatchKey::from(shortage),
            GoalDispatchKey::PostNoticeOther
        );
        assert_eq!(
            GoalDispatchKey::from(institutional),
            GoalDispatchKey::PostNoticeOther
        );
    }

    #[test]
    fn test_goal_dispatch_key_recipe_inputs_collapse_by_dispatch_shape() {
        let first = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Grain,
            purpose: CommodityPurpose::RecipeInput(RecipeId(1)),
            quantity: AcquisitionQuantity::single(),
        };
        let second = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::RecipeInput(RecipeId(99)),
            quantity: AcquisitionQuantity::single(),
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
    fn test_goal_dispatch_key_maps_expectation_goal_variants() {
        let subject = entity(11);
        let destination = entity(12);

        assert_eq!(
            GoalDispatchKey::from(GoalKind::SearchForMissing {
                subject,
                last_seen: Some(destination),
            }),
            GoalDispatchKey::SearchForMissing
        );
        assert_eq!(
            GoalDispatchKey::from(GoalKind::ReportMissing {
                subject,
                to_office: Some(destination),
                expectation_id: None,
            }),
            GoalDispatchKey::ReportMissing
        );
        assert_eq!(
            GoalDispatchKey::from(GoalKind::EscortToSafety {
                subject,
                destination,
            }),
            GoalDispatchKey::EscortToSafety
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
                quantity: AcquisitionQuantity::single(),
            },
            GoalKind::Sleep,
            GoalKind::Relieve,
            GoalKind::Wash,
            GoalKind::EngageHostile { target },
            GoalKind::RaidTarget { target },
            GoalKind::ReduceDanger,
            GoalKind::RegroupWithFaction { faction: office },
            GoalKind::TreatWounds { patient: target },
            GoalKind::SearchForMissing {
                subject: target,
                last_seen: Some(destination),
            },
            GoalKind::ReportMissing {
                subject: target,
                to_office: Some(office),
                expectation_id: None,
            },
            GoalKind::ReportFound {
                subject: target,
                expectation_id: ExpectationId(0),
            },
            GoalKind::EscortToSafety {
                subject: target,
                destination,
            },
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
            GoalKind::AskWitness {
                witness: target,
                topic: TellTopic::EntityBelief { subject: office },
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

        assert_eq!(goals.len(), 30);
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
        assert_eq!(GoalDispatchKey::all().len(), 41);
        for (idx, key) in GoalDispatchKey::all().iter().enumerate() {
            assert!(
                !GoalDispatchKey::all()[idx + 1..].contains(key),
                "duplicate key in exhaustive dispatch-key list: {key:?}"
            );
        }
        assert!(GoalDispatchKey::all().contains(&GoalDispatchKey::AskWitness));
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
