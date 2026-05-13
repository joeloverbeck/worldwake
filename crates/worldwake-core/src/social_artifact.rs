use crate::{CommodityKind, Component, EntityId, EventId, InstitutionalClaim, Quantity, Tick};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ArtifactHeader {
    pub kind: ArtifactKind,
    pub issuer: EntityId,
    pub issuing_authority: Option<EntityId>,
    pub created_at: Tick,
    pub expires_at: Option<Tick>,
    pub jurisdiction: Option<EntityId>,
    pub existence: ArtifactExistence,
    pub visibility: ArtifactVisibility,
    pub legal_effect: ArtifactLegalEffect,
    pub credibility: ArtifactCredibility,
    pub actionability: ArtifactActionability,
}

impl Component for ArtifactHeader {}

impl ArtifactHeader {
    pub fn posted_active(
        kind: ArtifactKind,
        issuer: EntityId,
        issuing_authority: Option<EntityId>,
        created_at: Tick,
        expires_at: Option<Tick>,
        jurisdiction: Option<EntityId>,
        place: EntityId,
    ) -> Self {
        Self {
            kind,
            issuer,
            issuing_authority,
            created_at,
            expires_at,
            jurisdiction,
            existence: ArtifactExistence::Exists,
            visibility: ArtifactVisibility::Posted { place },
            legal_effect: ArtifactLegalEffect::Active { expires_at },
            credibility: ArtifactCredibility::Credible,
            actionability: ArtifactActionability::Actionable,
        }
    }
}

/// Per-agent defaults for artifact TTL when posting notices and bounties.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactPostingProfile {
    /// Default time-to-live in ticks for threat warning notice artifacts.
    pub threat_warning_ttl: u64,
    /// Default time-to-live in ticks for office vacancy notice artifacts.
    pub office_vacancy_ttl: u64,
    /// Default time-to-live in ticks for bounty artifacts.
    pub bounty_ttl: u64,
}

impl Default for ArtifactPostingProfile {
    fn default() -> Self {
        Self {
            threat_warning_ttl: 48,
            office_vacancy_ttl: 96,
            bounty_ttl: 144,
        }
    }
}

impl Component for ArtifactPostingProfile {}

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ArtifactExistence {
    Exists,
    Destroyed {
        destroyed_at: Tick,
        cause: DestructionCause,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ArtifactVisibility {
    Hidden,
    Private { audience: BTreeSet<EntityId> },
    Posted { place: EntityId },
    WidelyKnown,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ArtifactLegalEffect {
    None,
    Active {
        expires_at: Option<Tick>,
    },
    Suspended {
        reason: SuspensionReason,
        suspended_at: Tick,
    },
    Expired {
        expired_at: Tick,
    },
    Revoked {
        revoked_at: Tick,
        by: EntityId,
        reason: RevocationReason,
    },
    Fulfilled {
        fulfilled_at: Tick,
        by: EntityId,
        evidence: EntityId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ArtifactCredibility {
    Credible,
    Disputed {
        disputed_at: Tick,
        contradicting: BTreeSet<EntityId>,
    },
    Refuted {
        refuted_at: Tick,
        evidence: EntityId,
    },
    Unknown,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ArtifactActionability {
    Actionable,
    AwaitingProof { required_proof: ProofKind },
    Blocked { reason: BlockerReason, since: Tick },
    Closed { closed_at: Tick, cause: CloseCause },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum DestructionCause {
    Adjudication,
    IssuerDestroyed,
    Superseded,
    Decay,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SuspensionReason {
    JurisdictionDispute,
    EvidenceWithheld,
    ProcessReview,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RevocationReason {
    IssuerWithdrawal,
    Adjudication,
    SupersededByLater,
}

/// Actionability-axis proof categories. `ProofRequirement` remains the bounty-terms field.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ProofKind {
    PhysicalEvidence,
    WitnessTestimony,
    SelfReport,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BlockerReason {
    LegalEffectExpired,
    LegalEffectRevoked,
    JurisdictionConflict,
    AwaitingAdjudication,
    BountyFulfilled,
    Adjudicated,
    Refuted,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CloseCause {
    BountyFulfilled,
    LegalEffectExpired,
    Revoked,
    Adjudicated,
    Refuted,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum AxisName {
    Existence,
    Visibility,
    LegalEffect,
    Credibility,
    Actionability,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ArtifactAxisValue {
    Existence(ArtifactExistence),
    Visibility(ArtifactVisibility),
    LegalEffect(ArtifactLegalEffect),
    Credibility(ArtifactCredibility),
    Actionability(ArtifactActionability),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ArtifactTransitionPayload {
    pub artifact: EntityId,
    pub axis: AxisName,
    pub prior: ArtifactAxisValue,
    pub new: ArtifactAxisValue,
    pub cause_event: Option<EventId>,
    pub at: Tick,
}

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BountyTerms {
    pub target: BountyTarget,
    pub proof_requirement: ProofRequirement,
    pub reward_commodity: CommodityKind,
    pub reward_quantity: Quantity,
    pub reward_source: RewardSource,
    pub claim_place: EntityId,
}

impl Component for BountyTerms {}

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
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

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ProofRequirement {
    PhysicalEvidence,
    WitnessTestimony,
    SelfReport,
}

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum RewardSource {
    InstitutionalTreasury { treasury_entity: EntityId },
    PersonalFunds { issuer: EntityId },
    ReservedLot { lot: EntityId },
}

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NoticeContent {
    pub topic: NoticeTopic,
}

impl Component for NoticeContent {}

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
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
        ArtifactActionability, ArtifactAxisValue, ArtifactCredibility, ArtifactExistence,
        ArtifactHeader, ArtifactKind, ArtifactLegalEffect, ArtifactPostingContext,
        ArtifactPostingProfile, ArtifactTransitionPayload, ArtifactVisibility, AxisName,
        BountyTarget, BountyTerms, NoticeContent, NoticeTopic, ProofRequirement, RewardSource,
    };
    use crate::{
        CommodityKind, EntityId, EventId, InstitutionalClaim, Quantity, Tick, traits::Component,
    };
    use serde::{Serialize, de::DeserializeOwned};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_traits<T: Clone + std::fmt::Debug + Eq + Serialize + DeserializeOwned>() {}

    #[test]
    fn social_artifact_types_satisfy_required_traits() {
        assert_component_bounds::<ArtifactPostingProfile>();
        assert_traits::<ArtifactHeader>();
        assert_traits::<ArtifactPostingProfile>();
        assert_traits::<ArtifactPostingContext>();
        assert_traits::<ArtifactKind>();
        assert_traits::<ArtifactExistence>();
        assert_traits::<ArtifactVisibility>();
        assert_traits::<ArtifactLegalEffect>();
        assert_traits::<ArtifactCredibility>();
        assert_traits::<ArtifactActionability>();
        assert_traits::<ArtifactTransitionPayload>();
        assert_traits::<BountyTerms>();
        assert_traits::<BountyTarget>();
        assert_traits::<ProofRequirement>();
        assert_traits::<RewardSource>();
        assert_traits::<NoticeContent>();
        assert_traits::<NoticeTopic>();
    }

    #[test]
    fn artifact_header_roundtrips_through_bincode() {
        let header = ArtifactHeader::posted_active(
            ArtifactKind::Bounty,
            entity(1),
            Some(entity(2)),
            Tick(3),
            Some(Tick(9)),
            Some(entity(4)),
            entity(5),
        );

        let bytes = bincode::serialize(&header).unwrap();
        let roundtrip: ArtifactHeader = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, header);
    }

    #[test]
    fn artifact_header_axis_defaults_match_migration_map() {
        let header = ArtifactHeader::posted_active(
            ArtifactKind::Notice,
            entity(1),
            None,
            Tick(3),
            Some(Tick(9)),
            None,
            entity(4),
        );

        assert_eq!(header.existence, ArtifactExistence::Exists);
        assert_eq!(
            header.visibility,
            ArtifactVisibility::Posted { place: entity(4) }
        );
        assert_eq!(
            header.legal_effect,
            ArtifactLegalEffect::Active {
                expires_at: Some(Tick(9))
            }
        );
        assert_eq!(header.credibility, ArtifactCredibility::Credible);
        assert_eq!(header.actionability, ArtifactActionability::Actionable);
    }

    #[test]
    fn artifact_transition_payload_roundtrips_through_bincode() {
        let payload = ArtifactTransitionPayload {
            artifact: entity(1),
            axis: AxisName::Actionability,
            prior: ArtifactAxisValue::Actionability(ArtifactActionability::Actionable),
            new: ArtifactAxisValue::Actionability(ArtifactActionability::Closed {
                closed_at: Tick(11),
                cause: super::CloseCause::LegalEffectExpired,
            }),
            cause_event: Some(EventId(7)),
            at: Tick(11),
        };

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: ArtifactTransitionPayload = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, payload);
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

    #[test]
    fn artifact_posting_profile_default_matches_spec_defaults() {
        let profile = ArtifactPostingProfile::default();

        assert_eq!(profile.threat_warning_ttl, 48);
        assert_eq!(profile.office_vacancy_ttl, 96);
        assert_eq!(profile.bounty_ttl, 144);
    }
}
