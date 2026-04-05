use crate::{CommodityKind, Component, EntityId, InstitutionalClaim, Quantity, Tick};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ArtifactHeader {
    pub kind: ArtifactKind,
    pub issuer: EntityId,
    pub issuing_authority: Option<EntityId>,
    pub created_at: Tick,
    pub expires_at: Option<Tick>,
    pub state: ArtifactState,
    pub jurisdiction: Option<EntityId>,
}

impl Component for ArtifactHeader {}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ArtifactPostingContext {
    pub posting_place: EntityId,
    pub issuing_authority: Option<EntityId>,
    pub expires_at: Option<Tick>,
    pub jurisdiction: Option<EntityId>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ArtifactKind {
    Bounty,
    Notice,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ArtifactState {
    Active,
    Fulfilled,
    Expired,
    Withdrawn,
    Destroyed,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BountyTerms {
    pub target: BountyTarget,
    pub proof_requirement: ProofRequirement,
    pub reward_commodity: CommodityKind,
    pub reward_quantity: Quantity,
    pub reward_source: RewardSource,
    pub claim_place: EntityId,
}

impl Component for BountyTerms {}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum BountyTarget {
    EliminateEntity {
        target: EntityId,
    },
    DeliverCommodity {
        commodity: CommodityKind,
        quantity: Quantity,
        destination: EntityId,
    },
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ProofRequirement {
    PhysicalEvidence,
    WitnessTestimony,
    SelfReport,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum RewardSource {
    InstitutionalTreasury { treasury_entity: EntityId },
    PersonalFunds { issuer: EntityId },
    ReservedLot { lot: EntityId },
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NoticeContent {
    pub topic: NoticeTopic,
}

impl Component for NoticeContent {}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum NoticeTopic {
    ThreatWarning {
        place: EntityId,
    },
    OfficeVacancy {
        office: EntityId,
    },
    CommodityShortage {
        commodity: CommodityKind,
        place: EntityId,
    },
    Institutional {
        claim: InstitutionalClaim,
    },
}

#[cfg(test)]
mod tests {
    use super::{
        ArtifactHeader, ArtifactKind, ArtifactPostingContext, ArtifactState, BountyTarget,
        BountyTerms, NoticeContent, NoticeTopic, ProofRequirement, RewardSource,
    };
    use crate::{CommodityKind, EntityId, InstitutionalClaim, Quantity, Tick};
    use serde::{de::DeserializeOwned, Serialize};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn assert_traits<T: Clone + std::fmt::Debug + Eq + Serialize + DeserializeOwned>() {}

    #[test]
    fn social_artifact_types_satisfy_required_traits() {
        assert_traits::<ArtifactHeader>();
        assert_traits::<ArtifactPostingContext>();
        assert_traits::<ArtifactKind>();
        assert_traits::<ArtifactState>();
        assert_traits::<BountyTerms>();
        assert_traits::<BountyTarget>();
        assert_traits::<ProofRequirement>();
        assert_traits::<RewardSource>();
        assert_traits::<NoticeContent>();
        assert_traits::<NoticeTopic>();
    }

    #[test]
    fn artifact_header_roundtrips_through_bincode() {
        let header = ArtifactHeader {
            kind: ArtifactKind::Bounty,
            issuer: entity(1),
            issuing_authority: Some(entity(2)),
            created_at: Tick(3),
            expires_at: Some(Tick(9)),
            state: ArtifactState::Active,
            jurisdiction: Some(entity(4)),
        };

        let bytes = bincode::serialize(&header).unwrap();
        let roundtrip: ArtifactHeader = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, header);
    }

    #[test]
    fn bounty_and_notice_types_roundtrip_through_bincode() {
        let posting = ArtifactPostingContext {
            posting_place: entity(4),
            issuing_authority: Some(entity(8)),
            expires_at: Some(Tick(20)),
            jurisdiction: Some(entity(9)),
        };
        let bounty = BountyTerms {
            target: BountyTarget::DeliverCommodity {
                commodity: CommodityKind::Bread,
                quantity: Quantity(7),
                destination: entity(5),
            },
            proof_requirement: ProofRequirement::WitnessTestimony,
            reward_commodity: CommodityKind::Coin,
            reward_quantity: Quantity(12),
            reward_source: RewardSource::InstitutionalTreasury {
                treasury_entity: entity(6),
            },
            claim_place: entity(7),
        };
        let notice = NoticeContent {
            topic: NoticeTopic::Institutional {
                claim: InstitutionalClaim::OfficeHolder {
                    office: entity(8),
                    holder: Some(entity(9)),
                    effective_tick: Tick(10),
                },
            },
        };

        let bounty_bytes = bincode::serialize(&bounty).unwrap();
        let bounty_roundtrip: BountyTerms = bincode::deserialize(&bounty_bytes).unwrap();
        assert_eq!(bounty_roundtrip, bounty);

        let posting_bytes = bincode::serialize(&posting).unwrap();
        let posting_roundtrip: ArtifactPostingContext =
            bincode::deserialize(&posting_bytes).unwrap();
        assert_eq!(posting_roundtrip, posting);

        let notice_bytes = bincode::serialize(&notice).unwrap();
        let notice_roundtrip: NoticeContent = bincode::deserialize(&notice_bytes).unwrap();
        assert_eq!(notice_roundtrip, notice);
    }
}
