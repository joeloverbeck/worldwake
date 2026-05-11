use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{
    BeliefRef, CommodityKind, EntityId, OpportunityKey, Permille, Tick, ViolationKind,
};
use worldwake_sim::EffectFact;

use crate::PlannerOpKind;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum EffectFactKey {
    CommodityTransfer,
    PartialQuantity,
    WoundApplied,
    ExpectationFulfilled,
    ContentionGrantConsumed,
    EventEmitted,
}

impl From<&EffectFact> for EffectFactKey {
    fn from(fact: &EffectFact) -> Self {
        match fact {
            EffectFact::CommodityTransfer { .. } => Self::CommodityTransfer,
            EffectFact::PartialQuantity { .. } => Self::PartialQuantity,
            EffectFact::WoundApplied { .. } => Self::WoundApplied,
            EffectFact::ExpectationFulfilled { .. } => Self::ExpectationFulfilled,
            EffectFact::ContentionGrantConsumed { .. } => Self::ContentionGrantConsumed,
            EffectFact::EventEmitted { .. } => Self::EventEmitted,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum RiskFact {
    CriminalLiability { violation_kind: ViolationKind },
    SocialShameRisk,
    ThreatPresence { source: EntityId },
    InjuryRisk,
    PropertyForfeitureRisk,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ClaimTopic {
    EntityLocation {
        subject: EntityId,
    },
    CommodityAvailability {
        commodity: CommodityKind,
        place: EntityId,
    },
    OwnershipClaim {
        item: EntityId,
    },
    HostilePresence {
        place: EntityId,
    },
    RouteSafety {
        from: EntityId,
        to: EntityId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum BelievedLegalStatus {
    BelievedOwned { owner: EntityId },
    BelievedUnclaimed,
    BelievedContested,
    SociallyOpenToRequest,
    Forbidden { jurisdiction: EntityId },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SocialExposureBand {
    Private,
    Public,
    PublicWithCriminalRisk,
    PublicWithShameRisk,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Opportunity {
    pub key: OpportunityKey,
    pub perceived_at: Tick,
    pub source_belief: BeliefRef,
    pub possible_effects: Vec<EffectFactKey>,
    pub possible_information: Vec<ClaimTopic>,
    pub required_actions: Vec<PlannerOpKind>,
    pub legal_status: BelievedLegalStatus,
    pub social_exposure: SocialExposureBand,
    pub risks: Vec<RiskFact>,
    pub salience: Permille,
}

#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub struct OpportunityHandle(pub u32);

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PerceivedOpportunityIndex {
    pub by_place: BTreeMap<EntityId, Vec<OpportunityHandle>>,
    pub by_anchor: BTreeMap<EntityId, OpportunityHandle>,
    pub all: Vec<Opportunity>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;
    use worldwake_core::{
        AcquisitionQuantity, BeliefClaimKey, BeliefStatusTag, CommodityPurpose, EntityBeliefAspect,
        GoalKind, OpportunityAnchor,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn sample_belief_ref() -> BeliefRef {
        BeliefRef {
            claim_key: BeliefClaimKey {
                subject: entity(1),
                aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
            },
            claim_held_at_tick: Tick(9),
            status: BeliefStatusTag::Probable,
        }
    }

    fn sample_opportunity() -> Opportunity {
        Opportunity {
            key: OpportunityKey {
                goal_key: GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                }
                .into(),
                anchor: OpportunityAnchor::Entity(entity(2)),
            },
            perceived_at: Tick(10),
            source_belief: sample_belief_ref(),
            possible_effects: vec![EffectFactKey::CommodityTransfer],
            possible_information: vec![ClaimTopic::OwnershipClaim { item: entity(2) }],
            required_actions: vec![PlannerOpKind::Trade],
            legal_status: BelievedLegalStatus::BelievedOwned { owner: entity(3) },
            social_exposure: SocialExposureBand::Public,
            risks: vec![RiskFact::SocialShameRisk],
            salience: Permille::new_unchecked(640),
        }
    }

    fn assert_bincode_roundtrip<T>(value: &T)
    where
        T: Debug + Eq + Serialize + DeserializeOwned,
    {
        let bytes = bincode::serialize(value).unwrap();
        let roundtrip: T = bincode::deserialize(&bytes).unwrap();
        assert_eq!(&roundtrip, value);
    }

    #[test]
    fn opportunity_compiler_types_roundtrip_through_bincode() {
        assert_bincode_roundtrip(&EffectFactKey::EventEmitted);
        assert_bincode_roundtrip(&RiskFact::CriminalLiability {
            violation_kind: ViolationKind::EntityDead { entity: entity(4) },
        });
        assert_bincode_roundtrip(&ClaimTopic::RouteSafety {
            from: entity(5),
            to: entity(6),
        });
        assert_bincode_roundtrip(&BelievedLegalStatus::Forbidden {
            jurisdiction: entity(7),
        });
        assert_bincode_roundtrip(&SocialExposureBand::PublicWithCriminalRisk);
        assert_bincode_roundtrip(&OpportunityHandle(3));

        let opportunity = sample_opportunity();
        assert_bincode_roundtrip(&opportunity);

        let mut index = PerceivedOpportunityIndex::default();
        index.by_place.insert(entity(8), vec![OpportunityHandle(0)]);
        index.by_anchor.insert(entity(2), OpportunityHandle(0));
        index.all.push(opportunity);
        assert_bincode_roundtrip(&index);
    }

    #[test]
    fn effect_fact_key_exhaustively_mirrors_effect_fact_variants() {
        fn key_for(fact: &EffectFact) -> EffectFactKey {
            EffectFactKey::from(fact)
        }

        assert_eq!(
            [
                EffectFactKey::CommodityTransfer,
                EffectFactKey::PartialQuantity,
                EffectFactKey::WoundApplied,
                EffectFactKey::ExpectationFulfilled,
                EffectFactKey::ContentionGrantConsumed,
                EffectFactKey::EventEmitted,
            ]
            .len(),
            6
        );

        let _ = key_for as fn(&EffectFact) -> EffectFactKey;
    }
}
