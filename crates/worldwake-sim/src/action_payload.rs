use serde::{Deserialize, Serialize};
use worldwake_core::{
    ActionDefId, BountyTarget, CombatWeaponRef, CommodityKind, EntityId, ExpectationId,
    NoticeTopic, ProofRequirement, PunishmentKind, Quantity, RecipeId, RecordEntryId, RewardSource,
    TellTopic, Tick, UniqueItemKind, ViolationId, WorkstationTag,
};

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ActionPayload {
    #[default]
    None,
    ConsultRecord(ConsultRecordActionPayload),
    Tell(TellActionPayload),
    Bribe(BribeActionPayload),
    Threaten(ThreatenActionPayload),
    Accuse(AccuseActionPayload),
    Punish(PunishActionPayload),
    EstablishCamp(EstablishCampActionPayload),
    DeclareSupport(DeclareSupportActionPayload),
    PressForceClaim(PressForceClaimActionPayload),
    YieldForceClaim(YieldForceClaimActionPayload),
    Transport(TransportActionPayload),
    Harvest(HarvestActionPayload),
    Craft(CraftActionPayload),
    Trade(TradeActionPayload),
    Combat(CombatActionPayload),
    Loot(LootActionPayload),
    Investigate(InvestigateActionPayload),
    AskWitness(AskWitnessPayload),
    AskAboutPerson(AskAboutPersonActionPayload),
    SearchPlace(SearchPlaceActionPayload),
    ReportMissing(ReportMissingActionPayload),
    QueueForFacilityUse(QueueForFacilityUsePayload),
    StaffMarket(StaffMarketPayload),
    PostBounty(PostBountyActionPayload),
    PostNotice(PostNoticeActionPayload),
}

impl ActionPayload {
    #[must_use]
    pub const fn as_bribe(&self) -> Option<&BribeActionPayload> {
        match self {
            Self::Bribe(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_threaten(&self) -> Option<&ThreatenActionPayload> {
        match self {
            Self::Threaten(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_accuse(&self) -> Option<&AccuseActionPayload> {
        match self {
            Self::Accuse(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_punish(&self) -> Option<&PunishActionPayload> {
        match self {
            Self::Punish(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_declare_support(&self) -> Option<&DeclareSupportActionPayload> {
        match self {
            Self::DeclareSupport(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_establish_camp(&self) -> Option<&EstablishCampActionPayload> {
        match self {
            Self::EstablishCamp(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_harvest(&self) -> Option<&HarvestActionPayload> {
        match self {
            Self::Harvest(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_transport(&self) -> Option<&TransportActionPayload> {
        match self {
            Self::Transport(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_craft(&self) -> Option<&CraftActionPayload> {
        match self {
            Self::Craft(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_trade(&self) -> Option<&TradeActionPayload> {
        match self {
            Self::Trade(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_combat(&self) -> Option<&CombatActionPayload> {
        match self {
            Self::Combat(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_loot(&self) -> Option<&LootActionPayload> {
        match self {
            Self::Loot(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_queue_for_facility_use(&self) -> Option<&QueueForFacilityUsePayload> {
        match self {
            Self::QueueForFacilityUse(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_tell(&self) -> Option<&TellActionPayload> {
        match self {
            Self::Tell(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_consult_record(&self) -> Option<&ConsultRecordActionPayload> {
        match self {
            Self::ConsultRecord(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_press_force_claim(&self) -> Option<&PressForceClaimActionPayload> {
        match self {
            Self::PressForceClaim(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_yield_force_claim(&self) -> Option<&YieldForceClaimActionPayload> {
        match self {
            Self::YieldForceClaim(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_investigate(&self) -> Option<&InvestigateActionPayload> {
        match self {
            Self::Investigate(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_ask_witness(&self) -> Option<&AskWitnessPayload> {
        match self {
            Self::AskWitness(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_ask_about_person(&self) -> Option<&AskAboutPersonActionPayload> {
        match self {
            Self::AskAboutPerson(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_search_place(&self) -> Option<&SearchPlaceActionPayload> {
        match self {
            Self::SearchPlace(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_report_missing(&self) -> Option<&ReportMissingActionPayload> {
        match self {
            Self::ReportMissing(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_staff_market(&self) -> Option<&StaffMarketPayload> {
        match self {
            Self::StaffMarket(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_post_bounty(&self) -> Option<&PostBountyActionPayload> {
        match self {
            Self::PostBounty(payload) => Some(payload),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_post_notice(&self) -> Option<&PostNoticeActionPayload> {
        match self {
            Self::PostNotice(payload) => Some(payload),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ConsultRecordActionPayload {
    pub record: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TellActionPayload {
    pub listener: EntityId,
    pub topic: TellTopic,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BribeActionPayload {
    pub target: EntityId,
    pub offered_commodity: CommodityKind,
    pub offered_quantity: Quantity,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ThreatenActionPayload {
    pub target: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AccuseActionPayload {
    pub violation_id: ViolationId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct PunishActionPayload {
    pub office: EntityId,
    pub accusation_entry: RecordEntryId,
    pub punishment: PunishmentKind,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct EstablishCampActionPayload {
    pub faction: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DeclareSupportActionPayload {
    pub office: EntityId,
    pub candidate: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct PressForceClaimActionPayload {
    pub office: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct YieldForceClaimActionPayload {
    pub office: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TransportActionPayload {
    pub quantity: Quantity,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HarvestActionPayload {
    pub recipe_id: RecipeId,
    pub required_workstation_tag: WorkstationTag,
    pub output_commodity: CommodityKind,
    pub output_quantity: Quantity,
    pub required_tool_kinds: Vec<UniqueItemKind>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CraftActionPayload {
    pub recipe_id: RecipeId,
    pub required_workstation_tag: WorkstationTag,
    pub inputs: Vec<(CommodityKind, Quantity)>,
    pub outputs: Vec<(CommodityKind, Quantity)>,
    pub required_tool_kinds: Vec<UniqueItemKind>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TradeActionPayload {
    pub counterparty: EntityId,
    pub sale_lot: EntityId,
    pub offered_commodity: CommodityKind,
    pub offered_quantity: Quantity,
    pub requested_quantity: Quantity,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CombatActionPayload {
    pub target: EntityId,
    pub weapon: CombatWeaponRef,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct LootActionPayload {
    pub target: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct InvestigateActionPayload {
    pub violation_id: ViolationId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AskWitnessPayload {
    pub target: EntityId,
    pub topic_entity: Option<EntityId>,
    pub topic_commodity: Option<CommodityKind>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AskAboutPersonActionPayload {
    pub target: EntityId,
    pub subject: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SearchPlaceActionPayload {
    pub subject: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ReportMissingActionPayload {
    pub expectation_id: ExpectationId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct QueueForFacilityUsePayload {
    pub intended_action: ActionDefId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct StaffMarketPayload {
    pub commodity: CommodityKind,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct PostBountyActionPayload {
    pub posting_place: EntityId,
    pub issuing_authority: Option<EntityId>,
    pub expires_at: Option<Tick>,
    pub jurisdiction: Option<EntityId>,
    pub target: BountyTarget,
    pub proof_requirement: ProofRequirement,
    pub reward_commodity: CommodityKind,
    pub reward_quantity: Quantity,
    pub reward_source: RewardSource,
    pub claim_place: EntityId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct PostNoticeActionPayload {
    pub posting_place: EntityId,
    pub issuing_authority: Option<EntityId>,
    pub expires_at: Option<Tick>,
    pub jurisdiction: Option<EntityId>,
    pub topic: NoticeTopic,
}

#[cfg(test)]
mod tests {
    use super::{
        AccuseActionPayload, ActionPayload, AskAboutPersonActionPayload, AskWitnessPayload,
        BribeActionPayload, CombatActionPayload, ConsultRecordActionPayload, CraftActionPayload,
        DeclareSupportActionPayload, EstablishCampActionPayload, HarvestActionPayload,
        InvestigateActionPayload, LootActionPayload, PostBountyActionPayload,
        PostNoticeActionPayload, PressForceClaimActionPayload, PunishActionPayload,
        QueueForFacilityUsePayload, ReportMissingActionPayload, SearchPlaceActionPayload,
        StaffMarketPayload, TellActionPayload, ThreatenActionPayload, TradeActionPayload,
        TransportActionPayload, YieldForceClaimActionPayload,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use worldwake_core::{
        ActionDefId, BountyTarget, CombatWeaponRef, CommodityKind, EntityId, ExpectationId,
        NoticeTopic, ProofRequirement, PunishmentKind, Quantity, RecipeId, RecordEntryId,
        RewardSource, TellTopic, Tick, UniqueItemKind, ViolationId, WorkstationTag,
    };

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn sample_payload() -> HarvestActionPayload {
        HarvestActionPayload {
            recipe_id: RecipeId(4),
            required_workstation_tag: WorkstationTag::OrchardRow,
            output_commodity: CommodityKind::Apple,
            output_quantity: Quantity(2),
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
        }
    }

    fn sample_tell_payload() -> TellActionPayload {
        TellActionPayload {
            listener: EntityId {
                slot: 5,
                generation: 0,
            },
            topic: TellTopic::EntityBelief {
                subject: EntityId {
                    slot: 8,
                    generation: 2,
                },
            },
        }
    }

    fn sample_consult_record_payload() -> ConsultRecordActionPayload {
        ConsultRecordActionPayload {
            record: EntityId {
                slot: 3,
                generation: 1,
            },
        }
    }

    fn sample_bribe_payload() -> BribeActionPayload {
        BribeActionPayload {
            target: EntityId {
                slot: 9,
                generation: 1,
            },
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(7),
        }
    }

    fn sample_threaten_payload() -> ThreatenActionPayload {
        ThreatenActionPayload {
            target: EntityId {
                slot: 12,
                generation: 3,
            },
        }
    }

    fn sample_declare_support_payload() -> DeclareSupportActionPayload {
        DeclareSupportActionPayload {
            office: EntityId {
                slot: 14,
                generation: 0,
            },
            candidate: EntityId {
                slot: 16,
                generation: 1,
            },
        }
    }

    fn sample_press_force_claim_payload() -> PressForceClaimActionPayload {
        PressForceClaimActionPayload {
            office: EntityId {
                slot: 18,
                generation: 0,
            },
        }
    }

    fn sample_yield_force_claim_payload() -> YieldForceClaimActionPayload {
        YieldForceClaimActionPayload {
            office: EntityId {
                slot: 19,
                generation: 1,
            },
        }
    }

    fn sample_craft_payload() -> CraftActionPayload {
        CraftActionPayload {
            recipe_id: RecipeId(7),
            required_workstation_tag: WorkstationTag::Mill,
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
        }
    }

    fn sample_trade_payload() -> TradeActionPayload {
        TradeActionPayload {
            counterparty: EntityId {
                slot: 11,
                generation: 0,
            },
            sale_lot: EntityId {
                slot: 50,
                generation: 0,
            },
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(4),
            requested_quantity: Quantity(2),
        }
    }

    fn sample_combat_payload() -> CombatActionPayload {
        CombatActionPayload {
            target: EntityId {
                slot: 17,
                generation: 2,
            },
            weapon: CombatWeaponRef::Unarmed,
        }
    }

    fn sample_loot_payload() -> LootActionPayload {
        LootActionPayload {
            target: EntityId {
                slot: 23,
                generation: 1,
            },
        }
    }

    fn sample_investigate_payload() -> InvestigateActionPayload {
        InvestigateActionPayload {
            violation_id: ViolationId(5),
        }
    }

    fn sample_accuse_payload() -> AccuseActionPayload {
        AccuseActionPayload {
            violation_id: ViolationId(8),
        }
    }

    fn sample_punish_payload() -> PunishActionPayload {
        PunishActionPayload {
            office: EntityId {
                slot: 10,
                generation: 0,
            },
            accusation_entry: RecordEntryId(3),
            punishment: PunishmentKind::Exile {
                from_faction: EntityId {
                    slot: 11,
                    generation: 0,
                },
            },
        }
    }

    fn sample_establish_camp_payload() -> EstablishCampActionPayload {
        EstablishCampActionPayload {
            faction: EntityId {
                slot: 12,
                generation: 0,
            },
        }
    }

    fn sample_queue_payload() -> QueueForFacilityUsePayload {
        QueueForFacilityUsePayload {
            intended_action: ActionDefId(19),
        }
    }

    fn sample_ask_witness_payload() -> AskWitnessPayload {
        AskWitnessPayload {
            target: EntityId {
                slot: 26,
                generation: 0,
            },
            topic_entity: Some(EntityId {
                slot: 27,
                generation: 1,
            }),
            topic_commodity: Some(CommodityKind::Water),
        }
    }

    fn sample_staff_market_payload() -> StaffMarketPayload {
        StaffMarketPayload {
            commodity: CommodityKind::Bread,
        }
    }

    fn sample_ask_about_person_payload() -> AskAboutPersonActionPayload {
        AskAboutPersonActionPayload {
            target: EntityId {
                slot: 44,
                generation: 0,
            },
            subject: EntityId {
                slot: 45,
                generation: 1,
            },
        }
    }

    fn sample_report_missing_payload() -> ReportMissingActionPayload {
        ReportMissingActionPayload {
            expectation_id: ExpectationId(42),
        }
    }

    fn sample_search_place_payload() -> SearchPlaceActionPayload {
        SearchPlaceActionPayload {
            subject: EntityId {
                slot: 46,
                generation: 0,
            },
        }
    }

    fn sample_post_bounty_payload() -> PostBountyActionPayload {
        PostBountyActionPayload {
            posting_place: EntityId {
                slot: 30,
                generation: 0,
            },
            issuing_authority: Some(EntityId {
                slot: 31,
                generation: 0,
            }),
            expires_at: Some(Tick(32)),
            jurisdiction: Some(EntityId {
                slot: 33,
                generation: 0,
            }),
            target: BountyTarget::EliminateEntity {
                target: EntityId {
                    slot: 34,
                    generation: 0,
                },
            },
            proof_requirement: ProofRequirement::PhysicalEvidence,
            reward_commodity: CommodityKind::Coin,
            reward_quantity: Quantity(35),
            reward_source: RewardSource::InstitutionalTreasury {
                treasury_entity: EntityId {
                    slot: 36,
                    generation: 0,
                },
            },
            claim_place: EntityId {
                slot: 37,
                generation: 0,
            },
        }
    }

    fn sample_post_notice_payload() -> PostNoticeActionPayload {
        PostNoticeActionPayload {
            posting_place: EntityId {
                slot: 38,
                generation: 0,
            },
            issuing_authority: None,
            expires_at: Some(Tick(39)),
            jurisdiction: Some(EntityId {
                slot: 40,
                generation: 0,
            }),
            topic: NoticeTopic::ThreatWarning {
                place: EntityId {
                    slot: 41,
                    generation: 0,
                },
            },
        }
    }

    fn sample_ask_witness_entity_only_payload() -> AskWitnessPayload {
        AskWitnessPayload {
            target: EntityId {
                slot: 28,
                generation: 3,
            },
            topic_entity: Some(EntityId {
                slot: 29,
                generation: 0,
            }),
            topic_commodity: None,
        }
    }

    #[test]
    fn action_payload_satisfies_required_traits() {
        assert_traits::<ActionPayload>();
        assert_traits::<ConsultRecordActionPayload>();
        assert_traits::<TellActionPayload>();
        assert_traits::<BribeActionPayload>();
        assert_traits::<ThreatenActionPayload>();
        assert_traits::<AccuseActionPayload>();
        assert_traits::<PunishActionPayload>();
        assert_traits::<EstablishCampActionPayload>();
        assert_traits::<DeclareSupportActionPayload>();
        assert_traits::<PressForceClaimActionPayload>();
        assert_traits::<YieldForceClaimActionPayload>();
        assert_traits::<TransportActionPayload>();
        assert_traits::<HarvestActionPayload>();
        assert_traits::<CraftActionPayload>();
        assert_traits::<TradeActionPayload>();
        assert_traits::<CombatActionPayload>();
        assert_traits::<LootActionPayload>();
        assert_traits::<InvestigateActionPayload>();
        assert_traits::<AskAboutPersonActionPayload>();
        assert_traits::<SearchPlaceActionPayload>();
        assert_traits::<ReportMissingActionPayload>();
        assert_traits::<QueueForFacilityUsePayload>();
        assert_traits::<AskWitnessPayload>();
        assert_traits::<StaffMarketPayload>();
        assert_traits::<PostBountyActionPayload>();
        assert_traits::<PostNoticeActionPayload>();
    }

    #[test]
    fn action_payload_default_is_none() {
        assert_eq!(ActionPayload::default(), ActionPayload::None);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn typed_accessors_cover_social_payload_variants() {
        let harvest = ActionPayload::Harvest(sample_payload());
        let consult = ActionPayload::ConsultRecord(sample_consult_record_payload());
        let tell = ActionPayload::Tell(sample_tell_payload());
        let bribe = ActionPayload::Bribe(sample_bribe_payload());
        let threaten = ActionPayload::Threaten(sample_threaten_payload());
        let accuse = ActionPayload::Accuse(sample_accuse_payload());
        let punish = ActionPayload::Punish(sample_punish_payload());
        let establish_camp = ActionPayload::EstablishCamp(sample_establish_camp_payload());
        let declare_support = ActionPayload::DeclareSupport(sample_declare_support_payload());
        let press_force_claim = ActionPayload::PressForceClaim(sample_press_force_claim_payload());
        let yield_force_claim = ActionPayload::YieldForceClaim(sample_yield_force_claim_payload());
        let ask_about_person = ActionPayload::AskAboutPerson(sample_ask_about_person_payload());
        let search_place = ActionPayload::SearchPlace(sample_search_place_payload());
        let report_missing = ActionPayload::ReportMissing(sample_report_missing_payload());

        assert_eq!(
            consult.as_consult_record(),
            Some(&sample_consult_record_payload())
        );
        assert_eq!(consult.as_tell(), None);
        assert_eq!(consult.as_bribe(), None);
        assert_eq!(consult.as_threaten(), None);
        assert_eq!(consult.as_establish_camp(), None);
        assert_eq!(consult.as_declare_support(), None);
        assert_eq!(consult.as_press_force_claim(), None);
        assert_eq!(consult.as_yield_force_claim(), None);
        assert_eq!(consult.as_harvest(), None);
        assert_eq!(consult.as_transport(), None);
        assert_eq!(consult.as_craft(), None);
        assert_eq!(consult.as_trade(), None);
        assert_eq!(consult.as_combat(), None);
        assert_eq!(consult.as_loot(), None);
        assert_eq!(consult.as_queue_for_facility_use(), None);

        assert_eq!(tell.as_consult_record(), None);
        assert_eq!(tell.as_tell(), Some(&sample_tell_payload()));
        assert_eq!(tell.as_bribe(), None);
        assert_eq!(tell.as_threaten(), None);
        assert_eq!(tell.as_establish_camp(), None);
        assert_eq!(tell.as_declare_support(), None);
        assert_eq!(tell.as_press_force_claim(), None);
        assert_eq!(tell.as_yield_force_claim(), None);
        assert_eq!(tell.as_harvest(), None);
        assert_eq!(tell.as_transport(), None);
        assert_eq!(tell.as_craft(), None);
        assert_eq!(tell.as_trade(), None);
        assert_eq!(tell.as_combat(), None);
        assert_eq!(tell.as_loot(), None);
        assert_eq!(tell.as_queue_for_facility_use(), None);

        assert_eq!(bribe.as_consult_record(), None);
        assert_eq!(bribe.as_tell(), None);
        assert_eq!(bribe.as_bribe(), Some(&sample_bribe_payload()));
        assert_eq!(bribe.as_threaten(), None);
        assert_eq!(bribe.as_establish_camp(), None);
        assert_eq!(bribe.as_declare_support(), None);
        assert_eq!(bribe.as_press_force_claim(), None);
        assert_eq!(bribe.as_yield_force_claim(), None);
        assert_eq!(bribe.as_harvest(), None);
        assert_eq!(bribe.as_transport(), None);
        assert_eq!(bribe.as_craft(), None);
        assert_eq!(bribe.as_trade(), None);

        assert_eq!(threaten.as_consult_record(), None);
        assert_eq!(threaten.as_tell(), None);
        assert_eq!(threaten.as_bribe(), None);
        assert_eq!(threaten.as_threaten(), Some(&sample_threaten_payload()));
        assert_eq!(threaten.as_accuse(), None);
        assert_eq!(threaten.as_establish_camp(), None);
        assert_eq!(threaten.as_declare_support(), None);
        assert_eq!(threaten.as_press_force_claim(), None);
        assert_eq!(threaten.as_yield_force_claim(), None);
        assert_eq!(threaten.as_harvest(), None);
        assert_eq!(threaten.as_transport(), None);
        assert_eq!(threaten.as_craft(), None);
        assert_eq!(threaten.as_trade(), None);
        assert_eq!(threaten.as_report_missing(), None);

        assert_eq!(accuse.as_consult_record(), None);
        assert_eq!(accuse.as_tell(), None);
        assert_eq!(accuse.as_bribe(), None);
        assert_eq!(accuse.as_threaten(), None);
        assert_eq!(accuse.as_accuse(), Some(&sample_accuse_payload()));
        assert_eq!(accuse.as_punish(), None);
        assert_eq!(accuse.as_establish_camp(), None);
        assert_eq!(accuse.as_report_missing(), None);

        assert_eq!(punish.as_consult_record(), None);
        assert_eq!(punish.as_tell(), None);
        assert_eq!(punish.as_bribe(), None);
        assert_eq!(punish.as_threaten(), None);
        assert_eq!(punish.as_accuse(), None);
        assert_eq!(punish.as_punish(), Some(&sample_punish_payload()));
        assert_eq!(punish.as_establish_camp(), None);
        assert_eq!(accuse.as_declare_support(), None);
        assert_eq!(accuse.as_press_force_claim(), None);
        assert_eq!(accuse.as_yield_force_claim(), None);
        assert_eq!(accuse.as_harvest(), None);
        assert_eq!(accuse.as_transport(), None);
        assert_eq!(accuse.as_craft(), None);
        assert_eq!(accuse.as_trade(), None);

        assert_eq!(establish_camp.as_consult_record(), None);
        assert_eq!(establish_camp.as_tell(), None);
        assert_eq!(establish_camp.as_bribe(), None);
        assert_eq!(establish_camp.as_threaten(), None);
        assert_eq!(establish_camp.as_accuse(), None);
        assert_eq!(establish_camp.as_punish(), None);
        assert_eq!(
            establish_camp.as_establish_camp(),
            Some(&sample_establish_camp_payload())
        );
        assert_eq!(establish_camp.as_declare_support(), None);
        assert_eq!(establish_camp.as_press_force_claim(), None);
        assert_eq!(establish_camp.as_yield_force_claim(), None);
        assert_eq!(establish_camp.as_harvest(), None);
        assert_eq!(establish_camp.as_transport(), None);
        assert_eq!(establish_camp.as_craft(), None);
        assert_eq!(establish_camp.as_trade(), None);

        assert_eq!(declare_support.as_consult_record(), None);
        assert_eq!(declare_support.as_tell(), None);
        assert_eq!(declare_support.as_bribe(), None);
        assert_eq!(declare_support.as_threaten(), None);
        assert_eq!(declare_support.as_accuse(), None);
        assert_eq!(
            declare_support.as_declare_support(),
            Some(&sample_declare_support_payload())
        );
        assert_eq!(declare_support.as_press_force_claim(), None);
        assert_eq!(declare_support.as_yield_force_claim(), None);
        assert_eq!(declare_support.as_harvest(), None);
        assert_eq!(declare_support.as_transport(), None);
        assert_eq!(declare_support.as_craft(), None);
        assert_eq!(declare_support.as_trade(), None);

        assert_eq!(harvest.as_consult_record(), None);
        assert_eq!(harvest.as_tell(), None);
        assert_eq!(harvest.as_bribe(), None);
        assert_eq!(harvest.as_threaten(), None);
        assert_eq!(harvest.as_accuse(), None);
        assert_eq!(harvest.as_declare_support(), None);
        assert_eq!(harvest.as_press_force_claim(), None);
        assert_eq!(harvest.as_yield_force_claim(), None);

        assert_eq!(press_force_claim.as_consult_record(), None);
        assert_eq!(press_force_claim.as_tell(), None);
        assert_eq!(press_force_claim.as_bribe(), None);
        assert_eq!(press_force_claim.as_threaten(), None);
        assert_eq!(press_force_claim.as_accuse(), None);
        assert_eq!(press_force_claim.as_declare_support(), None);
        assert_eq!(
            press_force_claim.as_press_force_claim(),
            Some(&sample_press_force_claim_payload())
        );
        assert_eq!(press_force_claim.as_yield_force_claim(), None);
        assert_eq!(press_force_claim.as_harvest(), None);

        assert_eq!(yield_force_claim.as_consult_record(), None);
        assert_eq!(yield_force_claim.as_tell(), None);
        assert_eq!(yield_force_claim.as_bribe(), None);
        assert_eq!(yield_force_claim.as_threaten(), None);
        assert_eq!(yield_force_claim.as_accuse(), None);
        assert_eq!(yield_force_claim.as_declare_support(), None);
        assert_eq!(yield_force_claim.as_press_force_claim(), None);
        assert_eq!(
            yield_force_claim.as_yield_force_claim(),
            Some(&sample_yield_force_claim_payload())
        );
        assert_eq!(yield_force_claim.as_harvest(), None);

        assert_eq!(ask_about_person.as_consult_record(), None);
        assert_eq!(ask_about_person.as_tell(), None);
        assert_eq!(ask_about_person.as_accuse(), None);
        assert_eq!(
            ask_about_person.as_ask_about_person(),
            Some(&sample_ask_about_person_payload())
        );
        assert_eq!(ask_about_person.as_ask_witness(), None);
        assert_eq!(ask_about_person.as_search_place(), None);
        assert_eq!(ask_about_person.as_report_missing(), None);

        assert_eq!(search_place.as_consult_record(), None);
        assert_eq!(search_place.as_tell(), None);
        assert_eq!(search_place.as_accuse(), None);
        assert_eq!(search_place.as_ask_about_person(), None);
        assert_eq!(
            search_place.as_search_place(),
            Some(&sample_search_place_payload())
        );
        assert_eq!(search_place.as_ask_witness(), None);
        assert_eq!(search_place.as_report_missing(), None);

        assert_eq!(report_missing.as_consult_record(), None);
        assert_eq!(report_missing.as_tell(), None);
        assert_eq!(report_missing.as_accuse(), None);
        assert_eq!(
            report_missing.as_report_missing(),
            Some(&sample_report_missing_payload())
        );
        assert_eq!(report_missing.as_ask_about_person(), None);
        assert_eq!(report_missing.as_search_place(), None);
        assert_eq!(report_missing.as_queue_for_facility_use(), None);
    }

    #[test]
    fn typed_accessors_cover_trade_and_production_payload_variants() {
        let transport = ActionPayload::Transport(TransportActionPayload {
            quantity: Quantity(3),
        });
        let harvest = ActionPayload::Harvest(sample_payload());
        let craft = ActionPayload::Craft(sample_craft_payload());
        let trade = ActionPayload::Trade(sample_trade_payload());

        assert_eq!(
            transport.as_transport(),
            Some(&TransportActionPayload {
                quantity: Quantity(3)
            })
        );
        assert_eq!(transport.as_consult_record(), None);
        assert_eq!(transport.as_bribe(), None);
        assert_eq!(transport.as_threaten(), None);
        assert_eq!(transport.as_declare_support(), None);
        assert_eq!(transport.as_press_force_claim(), None);
        assert_eq!(transport.as_yield_force_claim(), None);
        assert_eq!(transport.as_harvest(), None);
        assert_eq!(transport.as_craft(), None);
        assert_eq!(transport.as_trade(), None);

        assert_eq!(harvest.as_consult_record(), None);
        assert_eq!(harvest.as_harvest(), Some(&sample_payload()));
        assert_eq!(harvest.as_bribe(), None);
        assert_eq!(harvest.as_threaten(), None);
        assert_eq!(harvest.as_declare_support(), None);
        assert_eq!(harvest.as_press_force_claim(), None);
        assert_eq!(harvest.as_yield_force_claim(), None);
        assert_eq!(harvest.as_transport(), None);
        assert_eq!(harvest.as_craft(), None);
        assert_eq!(harvest.as_trade(), None);

        assert_eq!(craft.as_consult_record(), None);
        assert_eq!(craft.as_harvest(), None);
        assert_eq!(craft.as_bribe(), None);
        assert_eq!(craft.as_threaten(), None);
        assert_eq!(craft.as_declare_support(), None);
        assert_eq!(craft.as_press_force_claim(), None);
        assert_eq!(craft.as_yield_force_claim(), None);
        assert_eq!(craft.as_transport(), None);
        assert_eq!(craft.as_craft(), Some(&sample_craft_payload()));
        assert_eq!(craft.as_trade(), None);

        assert_eq!(trade.as_consult_record(), None);
        assert_eq!(trade.as_harvest(), None);
        assert_eq!(trade.as_bribe(), None);
        assert_eq!(trade.as_threaten(), None);
        assert_eq!(trade.as_declare_support(), None);
        assert_eq!(trade.as_press_force_claim(), None);
        assert_eq!(trade.as_yield_force_claim(), None);
        assert_eq!(trade.as_transport(), None);
        assert_eq!(trade.as_craft(), None);
        assert_eq!(trade.as_trade(), Some(&sample_trade_payload()));
    }

    #[test]
    fn typed_accessors_cover_combat_and_queue_payload_variants() {
        let combat = ActionPayload::Combat(sample_combat_payload());
        let loot = ActionPayload::Loot(sample_loot_payload());
        let investigate = ActionPayload::Investigate(sample_investigate_payload());
        let queue = ActionPayload::QueueForFacilityUse(sample_queue_payload());
        let ask_witness = ActionPayload::AskWitness(sample_ask_witness_payload());
        let ask_about_person = ActionPayload::AskAboutPerson(sample_ask_about_person_payload());
        let search_place = ActionPayload::SearchPlace(sample_search_place_payload());
        let report_missing = ActionPayload::ReportMissing(sample_report_missing_payload());
        let post_bounty = ActionPayload::PostBounty(sample_post_bounty_payload());
        let post_notice = ActionPayload::PostNotice(sample_post_notice_payload());

        assert_eq!(combat.as_consult_record(), None);
        assert_eq!(combat.as_tell(), None);
        assert_eq!(combat.as_harvest(), None);
        assert_eq!(combat.as_bribe(), None);
        assert_eq!(combat.as_threaten(), None);
        assert_eq!(combat.as_declare_support(), None);
        assert_eq!(combat.as_press_force_claim(), None);
        assert_eq!(combat.as_yield_force_claim(), None);
        assert_eq!(combat.as_transport(), None);
        assert_eq!(combat.as_craft(), None);
        assert_eq!(combat.as_trade(), None);
        assert_eq!(combat.as_combat(), Some(&sample_combat_payload()));

        assert_eq!(loot.as_consult_record(), None);
        assert_eq!(loot.as_tell(), None);
        assert_eq!(loot.as_harvest(), None);
        assert_eq!(loot.as_bribe(), None);
        assert_eq!(loot.as_threaten(), None);
        assert_eq!(loot.as_declare_support(), None);
        assert_eq!(loot.as_press_force_claim(), None);
        assert_eq!(loot.as_yield_force_claim(), None);
        assert_eq!(loot.as_transport(), None);
        assert_eq!(loot.as_craft(), None);
        assert_eq!(loot.as_trade(), None);
        assert_eq!(loot.as_combat(), None);
        assert_eq!(loot.as_loot(), Some(&sample_loot_payload()));
        assert_eq!(loot.as_investigate(), None);

        assert_eq!(investigate.as_consult_record(), None);
        assert_eq!(investigate.as_tell(), None);
        assert_eq!(investigate.as_harvest(), None);
        assert_eq!(investigate.as_bribe(), None);
        assert_eq!(investigate.as_threaten(), None);
        assert_eq!(investigate.as_declare_support(), None);
        assert_eq!(investigate.as_press_force_claim(), None);
        assert_eq!(investigate.as_yield_force_claim(), None);
        assert_eq!(investigate.as_transport(), None);
        assert_eq!(investigate.as_craft(), None);
        assert_eq!(investigate.as_trade(), None);
        assert_eq!(investigate.as_combat(), None);
        assert_eq!(investigate.as_loot(), None);
        assert_eq!(
            investigate.as_investigate(),
            Some(&sample_investigate_payload())
        );

        assert_eq!(queue.as_consult_record(), None);
        assert_eq!(queue.as_tell(), None);
        assert_eq!(queue.as_harvest(), None);
        assert_eq!(queue.as_bribe(), None);
        assert_eq!(queue.as_threaten(), None);
        assert_eq!(queue.as_declare_support(), None);
        assert_eq!(queue.as_press_force_claim(), None);
        assert_eq!(queue.as_yield_force_claim(), None);
        assert_eq!(queue.as_transport(), None);
        assert_eq!(queue.as_craft(), None);
        assert_eq!(queue.as_trade(), None);
        assert_eq!(queue.as_combat(), None);
        assert_eq!(queue.as_loot(), None);
        assert_eq!(queue.as_investigate(), None);
        assert_eq!(
            queue.as_queue_for_facility_use(),
            Some(&sample_queue_payload())
        );

        assert_eq!(ask_witness.as_consult_record(), None);
        assert_eq!(ask_witness.as_tell(), None);
        assert_eq!(ask_witness.as_harvest(), None);
        assert_eq!(ask_witness.as_bribe(), None);
        assert_eq!(ask_witness.as_threaten(), None);
        assert_eq!(ask_witness.as_declare_support(), None);
        assert_eq!(ask_witness.as_press_force_claim(), None);
        assert_eq!(ask_witness.as_yield_force_claim(), None);
        assert_eq!(ask_witness.as_transport(), None);
        assert_eq!(ask_witness.as_craft(), None);
        assert_eq!(ask_witness.as_trade(), None);
        assert_eq!(ask_witness.as_combat(), None);
        assert_eq!(ask_witness.as_loot(), None);
        assert_eq!(ask_witness.as_investigate(), None);
        assert_eq!(ask_witness.as_queue_for_facility_use(), None);
        assert_eq!(
            ask_witness.as_ask_witness(),
            Some(&sample_ask_witness_payload())
        );
        assert_eq!(ask_witness.as_ask_about_person(), None);
        assert_eq!(ask_witness.as_search_place(), None);
        assert_eq!(ask_witness.as_report_missing(), None);
        assert_eq!(ask_witness.as_post_bounty(), None);
        assert_eq!(ask_witness.as_post_notice(), None);

        assert_eq!(
            ask_about_person.as_ask_about_person(),
            Some(&sample_ask_about_person_payload())
        );
        assert_eq!(ask_about_person.as_ask_witness(), None);
        assert_eq!(ask_about_person.as_search_place(), None);
        assert_eq!(ask_about_person.as_report_missing(), None);

        assert_eq!(
            search_place.as_search_place(),
            Some(&sample_search_place_payload())
        );
        assert_eq!(search_place.as_ask_witness(), None);
        assert_eq!(search_place.as_ask_about_person(), None);
        assert_eq!(search_place.as_report_missing(), None);

        assert_eq!(
            report_missing.as_report_missing(),
            Some(&sample_report_missing_payload())
        );
        assert_eq!(report_missing.as_post_bounty(), None);
        assert_eq!(report_missing.as_ask_witness(), None);
        assert_eq!(report_missing.as_ask_about_person(), None);
        assert_eq!(report_missing.as_search_place(), None);

        assert_eq!(
            post_bounty.as_post_bounty(),
            Some(&sample_post_bounty_payload())
        );
        assert_eq!(post_bounty.as_report_missing(), None);
        assert_eq!(post_bounty.as_post_notice(), None);
        assert_eq!(post_bounty.as_trade(), None);

        assert_eq!(
            post_notice.as_post_notice(),
            Some(&sample_post_notice_payload())
        );
        assert_eq!(post_notice.as_report_missing(), None);
        assert_eq!(post_notice.as_post_bounty(), None);
        assert_eq!(post_notice.as_tell(), None);
    }

    #[test]
    fn typed_accessors_return_none_for_action_payload_none() {
        let none = ActionPayload::None;

        assert_eq!(none.as_consult_record(), None);
        assert_eq!(none.as_harvest(), None);
        assert_eq!(none.as_tell(), None);
        assert_eq!(none.as_bribe(), None);
        assert_eq!(none.as_threaten(), None);
        assert_eq!(none.as_establish_camp(), None);
        assert_eq!(none.as_declare_support(), None);
        assert_eq!(none.as_press_force_claim(), None);
        assert_eq!(none.as_yield_force_claim(), None);
        assert_eq!(none.as_transport(), None);
        assert_eq!(none.as_craft(), None);
        assert_eq!(none.as_trade(), None);
        assert_eq!(none.as_combat(), None);
        assert_eq!(none.as_loot(), None);
        assert_eq!(none.as_investigate(), None);
        assert_eq!(none.as_queue_for_facility_use(), None);
        assert_eq!(none.as_ask_witness(), None);
        assert_eq!(none.as_ask_about_person(), None);
        assert_eq!(none.as_search_place(), None);
        assert_eq!(none.as_report_missing(), None);
        assert_eq!(none.as_post_bounty(), None);
        assert_eq!(none.as_post_notice(), None);
    }

    #[test]
    fn action_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::EstablishCamp(sample_establish_camp_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn tell_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Tell(sample_tell_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn consult_record_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::ConsultRecord(sample_consult_record_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn bribe_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Bribe(sample_bribe_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn threaten_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Threaten(sample_threaten_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn declare_support_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::DeclareSupport(sample_declare_support_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn press_force_claim_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::PressForceClaim(sample_press_force_claim_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn yield_force_claim_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::YieldForceClaim(sample_yield_force_claim_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn craft_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Craft(sample_craft_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn trade_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Trade(sample_trade_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn combat_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Combat(sample_combat_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn loot_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Loot(sample_loot_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn investigate_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::Investigate(sample_investigate_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn queue_for_facility_use_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::QueueForFacilityUse(sample_queue_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn ask_witness_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::AskWitness(sample_ask_witness_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn ask_witness_entity_only_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::AskWitness(sample_ask_witness_entity_only_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn search_place_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::SearchPlace(sample_search_place_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn staff_market_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::StaffMarket(sample_staff_market_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn post_bounty_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::PostBounty(sample_post_bounty_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn post_notice_payload_roundtrips_through_bincode() {
        let payload = ActionPayload::PostNotice(sample_post_notice_payload());

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ActionPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn staff_market_accessor_returns_inner() {
        let payload = ActionPayload::StaffMarket(sample_staff_market_payload());
        assert_eq!(
            payload.as_staff_market(),
            Some(&sample_staff_market_payload())
        );
        assert_eq!(payload.as_trade(), None);
        assert_eq!(payload.as_harvest(), None);
        assert_eq!(payload.as_post_bounty(), None);
        assert_eq!(payload.as_post_notice(), None);

        assert_eq!(ActionPayload::None.as_staff_market(), None);
    }
}
