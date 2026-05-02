//! Shared goal identity types used across authoritative memory and AI planning.

use crate::{
    ArtifactPostingContext, BountyTerms, CommodityKind, CommunicationClass, EntityId,
    ExpectationId, HomeostaticNeedId, NoticeTopic, PunishmentKind, RecipeId, RecordEntryId,
    TellTopic, ViolationId,
};
use serde::{Deserialize, Serialize};
use std::num::{NonZeroU16, NonZeroU32};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum CommodityPurpose {
    SelfConsume,
    Restock,
    RecipeInput(RecipeId),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ExplorationMotivation {
    NeedDriven(HomeostaticNeedId),
    Proactive,
}

/// What an exploring agent expects to find at the target place.
/// Drives both ranking input and arrival-time hypothesis evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum HypothesisKind {
    MayContainCommodity { commodity: CommodityKind },
    MayContainLatrine,
    MayContainWashBasin,
    MayContainSleepSite,
    Proactive,
}

/// Quantity intent on an `AcquireCommodity` goal. The goal is satisfied
/// when the agent has obtained at least `desired_min` units; the planner
/// prefers plans projected to deliver `desired_target`. `horizon_ticks`
/// is consumed by the candidate emitter — it stops emitting when
/// `current_tick + horizon_ticks` no longer covers the projected
/// need-breach tick.
///
/// Invariant: `desired_min <= desired_target`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct AcquisitionQuantity {
    pub desired_min: NonZeroU16,
    pub desired_target: NonZeroU16,
    pub horizon_ticks: NonZeroU32,
}

impl AcquisitionQuantity {
    #[must_use]
    pub const fn single() -> Self {
        Self {
            desired_min: NonZeroU16::MIN,
            desired_target: NonZeroU16::MIN,
            horizon_ticks: NonZeroU32::new(200).unwrap(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum GoalKind {
    ConsumeOwnedCommodity {
        commodity: CommodityKind,
    },
    AcquireCommodity {
        commodity: CommodityKind,
        purpose: CommodityPurpose,
        quantity: AcquisitionQuantity,
    },
    Sleep,
    Relieve,
    Wash,
    /// Agent wants to free carry capacity by dropping low-value items.
    FreeCarryCapacity,
    EngageHostile {
        target: EntityId,
    },
    RaidTarget {
        target: EntityId,
    },
    ReduceDanger,
    RegroupWithFaction {
        faction: EntityId,
    },
    EstablishBanditCamp {
        faction: EntityId,
    },
    TreatWounds {
        patient: EntityId,
    },
    SearchForMissing {
        subject: EntityId,
        last_seen: Option<EntityId>,
    },
    ReportMissing {
        subject: EntityId,
        to_office: Option<EntityId>,
        expectation_id: Option<ExpectationId>,
    },
    ReportFound {
        subject: EntityId,
        expectation_id: ExpectationId,
    },
    EscortToSafety {
        subject: EntityId,
        destination: EntityId,
    },
    ProduceCommodity {
        recipe_id: RecipeId,
    },
    SellCommodity {
        commodity: CommodityKind,
    },
    RestockCommodity {
        commodity: CommodityKind,
    },
    MoveCargo {
        commodity: CommodityKind,
        destination: EntityId,
    },
    LootCorpse {
        corpse: EntityId,
    },
    BuryCorpse {
        corpse: EntityId,
        burial_site: EntityId,
    },
    FulfillBounty {
        bounty: EntityId,
    },
    PostBounty {
        posting: ArtifactPostingContext,
        terms: BountyTerms,
    },
    PostNotice {
        posting: ArtifactPostingContext,
        topic: NoticeTopic,
    },
    ShareBelief {
        listener: EntityId,
        topic: TellTopic,
        communication_class: CommunicationClass,
    },
    ClaimOffice {
        office: EntityId,
    },
    SupportCandidateForOffice {
        office: EntityId,
        candidate: EntityId,
    },
    /// Investigate a concrete recorded expectation violation.
    InvestigateViolation {
        violation_id: ViolationId,
        place: EntityId,
    },
    Patrol {
        place: EntityId,
    },
    ExploreLocation {
        target_place: EntityId,
        motivating_need: ExplorationMotivation,
        hypothesis: HypothesisKind,
    },
    StealItem {
        target_item: EntityId,
    },
    Accuse {
        crime_register: EntityId,
        accused: EntityId,
        violation_id: ViolationId,
    },
    PunishAccused {
        office: EntityId,
        accused: EntityId,
        accusation_entry: RecordEntryId,
        punishment: PunishmentKind,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct GoalKey {
    pub kind: GoalKind,
    pub commodity: Option<CommodityKind>,
    pub entity: Option<EntityId>,
    pub place: Option<EntityId>,
}

/// Concrete world-state anchor distinguishing one opportunity from another
/// for the same desire (`GoalKey`).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum OpportunityAnchor {
    Place(EntityId),
    Entity(EntityId),
    None,
}

/// Identifies a specific opportunity: a desire plus the concrete anchor
/// being pursued for that desire.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct OpportunityKey {
    pub goal_key: GoalKey,
    pub anchor: OpportunityAnchor,
}

impl GoalKey {
    pub fn new(kind: GoalKind) -> Self {
        Self::from(kind)
    }
}

impl From<GoalKind> for GoalKey {
    fn from(kind: GoalKind) -> Self {
        // Per S127 Design Goal 9: `quantity` does not participate in goal
        // identity. Normalize it to a canonical value so two acquisition
        // goals with the same commodity+purpose share a key regardless of
        // `desired_target` / `desired_min` / `horizon_ticks`.
        let kind = match kind {
            GoalKind::AcquireCommodity {
                commodity, purpose, ..
            } => GoalKind::AcquireCommodity {
                commodity,
                purpose,
                quantity: AcquisitionQuantity::single(),
            },
            other => other,
        };
        let (commodity, entity, place) = match kind {
            GoalKind::ConsumeOwnedCommodity { commodity }
            | GoalKind::AcquireCommodity { commodity, .. }
            | GoalKind::SellCommodity { commodity }
            | GoalKind::RestockCommodity { commodity } => (Some(commodity), None, None),
            GoalKind::EngageHostile { target }
            | GoalKind::RaidTarget { target }
            | GoalKind::TreatWounds { patient: target }
            | GoalKind::SearchForMissing {
                subject: target, ..
            }
            | GoalKind::ReportMissing {
                subject: target, ..
            }
            | GoalKind::ReportFound {
                subject: target, ..
            }
            | GoalKind::LootCorpse { corpse: target }
            | GoalKind::FulfillBounty { bounty: target }
            | GoalKind::ClaimOffice { office: target }
            | GoalKind::StealItem {
                target_item: target,
            }
            | GoalKind::PunishAccused {
                accused: target, ..
            } => (None, Some(target), None),
            GoalKind::Accuse {
                crime_register,
                accused,
                ..
            } => (None, Some(accused), Some(crime_register)),
            GoalKind::MoveCargo {
                commodity,
                destination,
            } => (Some(commodity), None, Some(destination)),
            GoalKind::EscortToSafety {
                subject,
                destination,
            } => (None, Some(subject), Some(destination)),
            GoalKind::BuryCorpse {
                corpse,
                burial_site,
            } => (None, Some(corpse), Some(burial_site)),
            GoalKind::PostBounty { posting, .. } | GoalKind::PostNotice { posting, .. } => {
                (None, None, Some(posting.posting_place))
            }
            GoalKind::ShareBelief { listener, .. } => (None, Some(listener), None),
            GoalKind::RegroupWithFaction { faction }
            | GoalKind::EstablishBanditCamp { faction } => (None, Some(faction), None),
            GoalKind::SupportCandidateForOffice { office, candidate } => {
                (None, Some(office), Some(candidate))
            }
            GoalKind::Sleep
            | GoalKind::Relieve
            | GoalKind::Wash
            | GoalKind::FreeCarryCapacity
            | GoalKind::ReduceDanger
            | GoalKind::ProduceCommodity { .. } => (None, None, None),
            GoalKind::InvestigateViolation { place, .. } | GoalKind::Patrol { place } => {
                (None, None, Some(place))
            }
            GoalKind::ExploreLocation { target_place, .. } => (None, None, Some(target_place)),
        };

        Self {
            kind,
            commodity,
            entity,
            place,
        }
    }
}

impl From<&GoalKind> for GoalKey {
    fn from(kind: &GoalKind) -> Self {
        Self::from(*kind)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AcquisitionQuantity, CommodityPurpose, ExplorationMotivation, GoalKey, GoalKind,
        HypothesisKind, OpportunityAnchor, OpportunityKey, TellTopic, ViolationId,
    };
    use crate::{
        ArtifactPostingContext, BountyTarget, BountyTerms, CommodityKind, CommunicationClass,
        ExpectationId, HomeostaticNeedId, NoticeTopic, ProofRequirement, PunishmentKind, Quantity,
        RecipeId, RewardSource, test_utils::entity_id,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::collections::BTreeMap;
    use std::fmt::Debug;

    fn assert_value_bounds<T: Clone + Eq + Ord + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn acquisition_quantity_single_invariant() {
        let q = AcquisitionQuantity::single();
        assert!(q.desired_min <= q.desired_target);
        assert_eq!(q.desired_min, q.desired_target);
        assert_eq!(q.horizon_ticks.get(), 200);
    }

    #[test]
    fn acquisition_quantity_bincode_roundtrip() {
        let q = AcquisitionQuantity {
            desired_min: std::num::NonZeroU16::new(2).unwrap(),
            desired_target: std::num::NonZeroU16::new(5).unwrap(),
            horizon_ticks: std::num::NonZeroU32::new(123).unwrap(),
        };

        let bytes = bincode::serialize(&q).unwrap();
        let roundtrip: AcquisitionQuantity = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, q);
    }

    #[test]
    fn goal_model_types_satisfy_value_bounds() {
        assert_value_bounds::<CommodityPurpose>();
        assert_value_bounds::<GoalKind>();
        assert_value_bounds::<GoalKey>();
        assert_value_bounds::<OpportunityAnchor>();
        assert_value_bounds::<OpportunityKey>();
    }

    #[test]
    fn opportunity_goal_identity_types_satisfy_value_bounds() {
        assert_value_bounds::<OpportunityAnchor>();
        assert_value_bounds::<OpportunityKey>();
    }

    #[test]
    fn goal_key_extracts_canonical_commodity_for_acquisition() {
        let key = GoalKey::from(&GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });

        assert_eq!(key.commodity, Some(CommodityKind::Apple));
        assert_eq!(key.entity, None);
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_ignores_quantity() {
        let q1 = AcquisitionQuantity {
            desired_min: std::num::NonZeroU16::new(1).unwrap(),
            desired_target: std::num::NonZeroU16::new(2).unwrap(),
            horizon_ticks: std::num::NonZeroU32::new(50).unwrap(),
        };
        let q2 = AcquisitionQuantity {
            desired_min: std::num::NonZeroU16::new(3).unwrap(),
            desired_target: std::num::NonZeroU16::new(7).unwrap(),
            horizon_ticks: std::num::NonZeroU32::new(180).unwrap(),
        };
        assert_ne!(q1, q2);
        let a = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: q1,
        });
        let b = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: q2,
        });

        assert_eq!(a, b);
    }

    #[test]
    fn goal_key_extracts_canonical_entity_for_loot_corpse() {
        let corpse = entity_id(8, 1);
        let key = GoalKey::from(&GoalKind::LootCorpse { corpse });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(corpse));
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_extracts_patient_for_treat_wounds() {
        let patient = entity_id(20, 0);
        let key = GoalKey::from(GoalKind::TreatWounds { patient });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(patient));
        assert_eq!(key.place, None);
    }

    #[test]
    fn treat_wounds_goal_roundtrips_through_bincode() {
        let patient = entity_id(21, 0);
        let goal = GoalKind::TreatWounds { patient };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn free_carry_capacity_goal_roundtrips_through_bincode() {
        let goal = GoalKind::FreeCarryCapacity;

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn free_carry_capacity_goal_key_has_no_canonical_fields() {
        let key = GoalKey::from(GoalKind::FreeCarryCapacity);

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, None);
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_search_for_missing_keys_on_subject_only() {
        let subject = entity_id(22, 0);
        let key = GoalKey::from(GoalKind::SearchForMissing {
            subject,
            last_seen: Some(entity_id(23, 0)),
        });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(subject));
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_report_missing_keys_on_subject_only() {
        let subject = entity_id(24, 0);
        let key = GoalKey::from(GoalKind::ReportMissing {
            subject,
            to_office: Some(entity_id(25, 0)),
            expectation_id: Some(ExpectationId(7)),
        });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(subject));
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_escort_to_safety_extracts_subject_and_destination() {
        let subject = entity_id(26, 0);
        let destination = entity_id(27, 0);
        let key = GoalKey::from(GoalKind::EscortToSafety {
            subject,
            destination,
        });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(subject));
        assert_eq!(key.place, Some(destination));
    }

    #[test]
    fn escort_to_safety_goal_roundtrips_through_bincode() {
        let goal = GoalKind::EscortToSafety {
            subject: entity_id(28, 0),
            destination: entity_id(29, 0),
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn goal_key_extracts_posting_place_for_post_bounty() {
        let posting_place = entity_id(24, 0);
        let key = GoalKey::from(GoalKind::PostBounty {
            posting: ArtifactPostingContext {
                posting_place,
                issuing_authority: None,
                expires_at: None,
                jurisdiction: None,
            },
            terms: BountyTerms {
                target: BountyTarget::EliminateEntity {
                    target: entity_id(25, 0),
                },
                proof_requirement: ProofRequirement::SelfReport,
                reward_commodity: CommodityKind::Coin,
                reward_quantity: Quantity(5),
                reward_source: RewardSource::PersonalFunds {
                    issuer: entity_id(26, 0),
                },
                claim_place: posting_place,
            },
        });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, None);
        assert_eq!(key.place, Some(posting_place));
    }

    #[test]
    fn goal_key_preserves_notice_topic_in_kind_and_extracts_posting_place() {
        let posting_place = entity_id(26, 0);
        let goal = GoalKind::PostNotice {
            posting: ArtifactPostingContext {
                posting_place,
                issuing_authority: None,
                expires_at: None,
                jurisdiction: None,
            },
            topic: NoticeTopic::ThreatWarning {
                place: entity_id(27, 0),
            },
        };
        let key = GoalKey::from(goal);

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, None);
        assert_eq!(key.place, Some(posting_place));
        assert_eq!(key.kind, goal);
    }

    #[test]
    fn goal_key_extracts_canonical_entity_for_engage_hostile() {
        let target = entity_id(8, 2);
        let key = GoalKey::from(&GoalKind::EngageHostile { target });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(target));
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_extracts_canonical_entity_for_raid_target() {
        let target = entity_id(8, 3);
        let key = GoalKey::from(&GoalKind::RaidTarget { target });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(target));
        assert_eq!(key.place, None);
    }

    #[test]
    fn raid_target_goal_roundtrips_through_bincode() {
        let target = entity_id(21, 1);
        let goal = GoalKind::RaidTarget { target };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn goal_key_extracts_entity_and_place_for_move_cargo() {
        let destination = entity_id(9, 2);
        let key = GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Water,
            destination,
        });

        assert_eq!(key.commodity, Some(CommodityKind::Water));
        assert_eq!(key.entity, None);
        assert_eq!(key.place, Some(destination));
    }

    #[test]
    fn move_cargo_goal_identity_depends_on_commodity_and_destination_not_lot_identity() {
        let destination = entity_id(9, 2);
        let first = GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Water,
            destination,
        });
        let second = GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Water,
            destination,
        });

        assert_eq!(first.commodity, second.commodity);
        assert_eq!(first.entity, None);
        assert_eq!(second.entity, None);
        assert_eq!(first.place, second.place);
        assert_eq!(first, second);
    }

    #[test]
    fn goal_key_sleep_has_no_canonical_target_fields() {
        let key = GoalKey::from(&GoalKind::Sleep);

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, None);
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_roundtrips_through_bincode() {
        let key = GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: RecipeId(12),
        });

        let bytes = bincode::serialize(&key).unwrap();
        let roundtrip: GoalKey = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, key);
    }

    #[test]
    fn opportunity_anchor_ordering_is_stable() {
        let place = entity_id(40, 0);
        let entity = entity_id(41, 0);
        let mut anchors = vec![
            OpportunityAnchor::Entity(entity),
            OpportunityAnchor::None,
            OpportunityAnchor::Place(place),
        ];

        anchors.sort();

        assert_eq!(
            anchors,
            vec![
                OpportunityAnchor::Place(place),
                OpportunityAnchor::Entity(entity),
                OpportunityAnchor::None,
            ]
        );
    }

    #[test]
    fn opportunity_key_bincode_roundtrip_preserves_anchor_and_goal() {
        let key = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            anchor: OpportunityAnchor::Place(entity_id(42, 0)),
        };

        let bytes = bincode::serialize(&key).unwrap();
        let roundtrip: OpportunityKey = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, key);
    }

    #[test]
    fn opportunity_key_btreemap_iteration_is_deterministic() {
        let sleep = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            anchor: OpportunityAnchor::None,
        };
        let acquire_at_market = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            anchor: OpportunityAnchor::Place(entity_id(43, 0)),
        };
        let treat_patient = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::TreatWounds {
                patient: entity_id(44, 0),
            }),
            anchor: OpportunityAnchor::Entity(entity_id(44, 0)),
        };

        let keys = BTreeMap::from([
            (sleep, "sleep"),
            (treat_patient, "treat"),
            (acquire_at_market, "acquire"),
        ])
        .into_keys()
        .collect::<Vec<_>>();

        assert_eq!(keys, vec![acquire_at_market, sleep, treat_patient]);
    }

    #[test]
    fn goal_key_extracts_listener_and_subject_for_share_belief() {
        let listener = entity_id(11, 0);
        let subject = entity_id(12, 0);
        let key = GoalKey::from(GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
            communication_class: CommunicationClass::Gossip,
        });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(listener));
        assert_eq!(key.place, None);
    }

    #[test]
    fn share_belief_goal_identity_distinguishes_listener_and_subject() {
        let listener_a = entity_id(13, 0);
        let listener_b = entity_id(13, 1);
        let subject_a = entity_id(14, 0);
        let subject_b = entity_id(14, 1);

        let first = GoalKey::from(GoalKind::ShareBelief {
            listener: listener_a,
            topic: TellTopic::EntityBelief { subject: subject_a },
            communication_class: CommunicationClass::Gossip,
        });
        let second = GoalKey::from(GoalKind::ShareBelief {
            listener: listener_b,
            topic: TellTopic::EntityBelief { subject: subject_a },
            communication_class: CommunicationClass::Gossip,
        });
        let third = GoalKey::from(GoalKind::ShareBelief {
            listener: listener_a,
            topic: TellTopic::EntityBelief { subject: subject_b },
            communication_class: CommunicationClass::Gossip,
        });

        assert_ne!(first, second);
        assert_ne!(first, third);
        assert_eq!(first.entity, Some(listener_a));
        assert_eq!(first.place, None);
    }

    #[test]
    fn claim_office_goal_roundtrips_through_bincode() {
        let office = entity_id(15, 0);
        let goal = GoalKind::ClaimOffice { office };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn support_candidate_goal_roundtrips_through_bincode() {
        let office = entity_id(16, 0);
        let candidate = entity_id(16, 1);
        let goal = GoalKind::SupportCandidateForOffice { office, candidate };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn goal_key_extracts_office_for_claim_office() {
        let office = entity_id(17, 0);
        let key = GoalKey::from(GoalKind::ClaimOffice { office });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(office));
        assert_eq!(key.place, None);
    }

    #[test]
    fn goal_key_extracts_office_and_candidate_for_support_candidate() {
        let office = entity_id(18, 0);
        let candidate = entity_id(18, 1);
        let key = GoalKey::from(GoalKind::SupportCandidateForOffice { office, candidate });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(office));
        assert_eq!(key.place, Some(candidate));
    }

    #[test]
    fn goal_key_extracts_faction_for_regroup_with_faction() {
        let faction = entity_id(18, 2);
        let key = GoalKey::from(GoalKind::RegroupWithFaction { faction });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(faction));
        assert_eq!(key.place, None);
    }

    #[test]
    fn regroup_with_faction_goal_roundtrips_through_bincode() {
        let faction = entity_id(18, 3);
        let goal = GoalKind::RegroupWithFaction { faction };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn goal_key_extracts_place_for_investigate_violation() {
        let place = entity_id(22, 0);
        let key = GoalKey::from(GoalKind::InvestigateViolation {
            violation_id: ViolationId(7),
            place,
        });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, None);
        assert_eq!(key.place, Some(place));
    }

    #[test]
    fn investigate_violation_goal_roundtrips_through_bincode() {
        let place = entity_id(23, 0);
        let goal = GoalKind::InvestigateViolation {
            violation_id: ViolationId(8),
            place,
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn goal_key_extracts_place_for_patrol() {
        let place = entity_id(23, 1);
        let key = GoalKey::from(GoalKind::Patrol { place });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, None);
        assert_eq!(key.place, Some(place));
    }

    #[test]
    fn patrol_goal_roundtrips_through_bincode() {
        let goal = GoalKind::Patrol {
            place: entity_id(23, 2),
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn goal_key_extracts_target_place_for_explore_location() {
        let target_place = entity_id(23, 3);
        let key = GoalKey::from(GoalKind::ExploreLocation {
            target_place,
            motivating_need: ExplorationMotivation::NeedDriven(HomeostaticNeedId::Thirst),
            hypothesis: HypothesisKind::MayContainCommodity {
                commodity: CommodityKind::Water,
            },
        });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, None);
        assert_eq!(key.place, Some(target_place));
    }

    #[test]
    fn explore_location_goal_roundtrips_through_bincode() {
        let goal = GoalKind::ExploreLocation {
            target_place: entity_id(23, 4),
            motivating_need: ExplorationMotivation::NeedDriven(HomeostaticNeedId::Hunger),
            hypothesis: HypothesisKind::MayContainCommodity {
                commodity: CommodityKind::Apple,
            },
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn exploration_motivation_roundtrips_through_bincode() {
        let motivation = ExplorationMotivation::NeedDriven(HomeostaticNeedId::Thirst);

        let bytes = bincode::serialize(&motivation).unwrap();
        let roundtrip: ExplorationMotivation = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, motivation);
        assert_eq!(
            ExplorationMotivation::Proactive,
            ExplorationMotivation::Proactive
        );
    }

    #[test]
    fn hypothesis_kind_roundtrips_through_bincode() {
        let hypotheses = [
            HypothesisKind::MayContainCommodity {
                commodity: CommodityKind::Apple,
            },
            HypothesisKind::MayContainLatrine,
            HypothesisKind::MayContainWashBasin,
            HypothesisKind::MayContainSleepSite,
            HypothesisKind::Proactive,
        ];

        for hypothesis in hypotheses {
            let bytes = bincode::serialize(&hypothesis).unwrap();
            let roundtrip: HypothesisKind = bincode::deserialize(&bytes).unwrap();

            assert_eq!(roundtrip, hypothesis);
        }
    }

    #[test]
    fn goal_key_distinguishes_explore_location_by_hypothesis() {
        let target_place = entity_id(23, 5);
        let food_key = GoalKey::from(GoalKind::ExploreLocation {
            target_place,
            motivating_need: ExplorationMotivation::NeedDriven(HomeostaticNeedId::Hunger),
            hypothesis: HypothesisKind::MayContainCommodity {
                commodity: CommodityKind::Apple,
            },
        });
        let water_key = GoalKey::from(GoalKind::ExploreLocation {
            target_place,
            motivating_need: ExplorationMotivation::NeedDriven(HomeostaticNeedId::Hunger),
            hypothesis: HypothesisKind::MayContainCommodity {
                commodity: CommodityKind::Water,
            },
        });

        assert_ne!(food_key, water_key);
        assert_eq!(food_key.place, Some(target_place));
        assert_eq!(water_key.place, Some(target_place));
    }

    #[test]
    fn goal_key_extracts_target_item_for_steal_item() {
        let target_item = entity_id(24, 0);
        let key = GoalKey::from(GoalKind::StealItem { target_item });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(target_item));
        assert_eq!(key.place, None);
    }

    #[test]
    fn steal_item_goal_roundtrips_through_bincode() {
        let goal = GoalKind::StealItem {
            target_item: entity_id(25, 0),
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn goal_key_extracts_accused_for_accuse() {
        let accused = entity_id(26, 0);
        let crime_register = entity_id(25, 0);
        let key = GoalKey::from(GoalKind::Accuse {
            crime_register,
            accused,
            violation_id: ViolationId(9),
        });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(accused));
        assert_eq!(key.place, Some(crime_register));
    }

    #[test]
    fn accuse_goal_roundtrips_through_bincode() {
        let goal = GoalKind::Accuse {
            crime_register: entity_id(26, 0),
            accused: entity_id(27, 0),
            violation_id: ViolationId(10),
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn goal_key_extracts_accused_for_punish_accused() {
        let accused = entity_id(28, 0);
        let key = GoalKey::from(GoalKind::PunishAccused {
            office: entity_id(27, 0),
            accused,
            accusation_entry: crate::RecordEntryId(3),
            punishment: PunishmentKind::Exile {
                from_faction: entity_id(29, 0),
            },
        });

        assert_eq!(key.commodity, None);
        assert_eq!(key.entity, Some(accused));
        assert_eq!(key.place, None);
    }

    #[test]
    fn punish_accused_goal_roundtrips_through_bincode_for_fine_and_exile() {
        let fine = GoalKind::PunishAccused {
            office: entity_id(29, 0),
            accused: entity_id(30, 0),
            accusation_entry: crate::RecordEntryId(4),
            punishment: PunishmentKind::Fine {
                commodity: CommodityKind::Coin,
                amount: Quantity(5),
            },
        };
        let exile = GoalKind::PunishAccused {
            office: entity_id(30, 0),
            accused: entity_id(31, 0),
            accusation_entry: crate::RecordEntryId(5),
            punishment: PunishmentKind::Exile {
                from_faction: entity_id(32, 0),
            },
        };

        let fine_bytes = bincode::serialize(&fine).unwrap();
        let fine_roundtrip: GoalKind = bincode::deserialize(&fine_bytes).unwrap();
        assert_eq!(fine_roundtrip, fine);

        let exile_bytes = bincode::serialize(&exile).unwrap();
        let exile_roundtrip: GoalKind = bincode::deserialize(&exile_bytes).unwrap();
        assert_eq!(exile_roundtrip, exile);
    }

    #[test]
    fn support_candidate_goal_identity_distinguishes_candidate() {
        let office = entity_id(19, 0);
        let candidate_a = entity_id(19, 1);
        let candidate_b = entity_id(19, 2);

        let first = GoalKey::from(GoalKind::SupportCandidateForOffice {
            office,
            candidate: candidate_a,
        });
        let second = GoalKey::from(GoalKind::SupportCandidateForOffice {
            office,
            candidate: candidate_b,
        });

        assert_ne!(first, second);
        assert_eq!(first.entity, Some(office));
        assert_eq!(first.place, Some(candidate_a));
    }
}
