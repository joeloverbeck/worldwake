use std::collections::BTreeSet;

use worldwake_core::{
    AcquisitionQuantity, ArtifactPostingContext, BountyTarget, BountyTerms, CommodityKind,
    CommodityPurpose, CommunicationClass, EntityId, ExplorationMotivation, GoalKey, GoalKind,
    GoalKindDiscriminant, HomeostaticNeedId, HypothesisKind, MethodSchemaId, MotiveSource,
    MotiveSourceDiscriminant, NoticeTopic, OpportunityAnchor, OpportunityKey, ProofRequirement,
    PunishmentKind, Quantity, RecipeId, RecordEntryId, RewardSource, TellTopic, ViolationId,
    WoundId,
};

fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn opportunity() -> OpportunityKey {
    OpportunityKey {
        goal_key: GoalKey::new(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        anchor: OpportunityAnchor::Place(entity(100)),
    }
}

fn posting(place: EntityId) -> ArtifactPostingContext {
    ArtifactPostingContext {
        posting_place: place,
        issuing_authority: None,
        expires_at: None,
        jurisdiction: None,
    }
}

fn bounty_terms(target: EntityId, claim_place: EntityId) -> BountyTerms {
    BountyTerms {
        target: BountyTarget::EliminateEntity { target },
        proof_requirement: ProofRequirement::SelfReport,
        reward_commodity: CommodityKind::Coin,
        reward_quantity: Quantity(5),
        reward_source: RewardSource::PersonalFunds {
            issuer: entity(101),
        },
        claim_place,
    }
}

fn motive_source_examples() -> Vec<(MotiveSource, MotiveSourceDiscriminant)> {
    vec![
        (
            MotiveSource::NeedPressure {
                need: HomeostaticNeedId::Hunger,
            },
            MotiveSourceDiscriminant::NeedPressure,
        ),
        (
            MotiveSource::Pain { wound: WoundId(1) },
            MotiveSourceDiscriminant::Pain,
        ),
        (
            MotiveSource::OfficeDuty { office: entity(2) },
            MotiveSourceDiscriminant::OfficeDuty,
        ),
        (
            MotiveSource::Loyalty { other: entity(3) },
            MotiveSourceDiscriminant::Loyalty,
        ),
        (
            MotiveSource::Greed {
                opportunity: opportunity(),
            },
            MotiveSourceDiscriminant::Greed,
        ),
        (
            MotiveSource::Shame {
                reputation_record: entity(4),
            },
            MotiveSourceDiscriminant::Shame,
        ),
        (
            MotiveSource::Revenge {
                violation: ViolationId(5),
            },
            MotiveSourceDiscriminant::Revenge,
        ),
    ]
}

fn goal_kind_examples() -> Vec<(GoalKind, GoalKindDiscriminant)> {
    vec![
        (
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalKindDiscriminant::ConsumeOwnedCommodity,
        ),
        (
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            },
            GoalKindDiscriminant::AcquireCommodity,
        ),
        (GoalKind::Sleep, GoalKindDiscriminant::Sleep),
        (GoalKind::Relieve, GoalKindDiscriminant::Relieve),
        (GoalKind::Wash, GoalKindDiscriminant::Wash),
        (
            GoalKind::FreeCarryCapacity,
            GoalKindDiscriminant::FreeCarryCapacity,
        ),
        (
            GoalKind::EngageHostile { target: entity(10) },
            GoalKindDiscriminant::EngageHostile,
        ),
        (
            GoalKind::RaidTarget { target: entity(11) },
            GoalKindDiscriminant::RaidTarget,
        ),
        (GoalKind::ReduceDanger, GoalKindDiscriminant::ReduceDanger),
        (
            GoalKind::RegroupWithFaction {
                faction: entity(12),
            },
            GoalKindDiscriminant::RegroupWithFaction,
        ),
        (
            GoalKind::EstablishBanditCamp {
                faction: entity(13),
            },
            GoalKindDiscriminant::EstablishBanditCamp,
        ),
        (
            GoalKind::TreatWounds {
                patient: entity(14),
            },
            GoalKindDiscriminant::TreatWounds,
        ),
        (
            GoalKind::SearchForMissing {
                subject: entity(15),
                last_seen: Some(entity(16)),
            },
            GoalKindDiscriminant::SearchForMissing,
        ),
        (
            GoalKind::ReportMissing {
                subject: entity(17),
                to_office: Some(entity(18)),
                expectation_id: None,
            },
            GoalKindDiscriminant::ReportMissing,
        ),
        (
            GoalKind::ReportFound {
                subject: entity(19),
                expectation_id: worldwake_core::ExpectationId(1),
            },
            GoalKindDiscriminant::ReportFound,
        ),
        (
            GoalKind::EscortToSafety {
                subject: entity(20),
                destination: entity(21),
            },
            GoalKindDiscriminant::EscortToSafety,
        ),
        (
            GoalKind::ProduceCommodity {
                recipe_id: RecipeId(1),
            },
            GoalKindDiscriminant::ProduceCommodity,
        ),
        (
            GoalKind::SellCommodity {
                commodity: CommodityKind::Sword,
            },
            GoalKindDiscriminant::SellCommodity,
        ),
        (
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Water,
            },
            GoalKindDiscriminant::RestockCommodity,
        ),
        (
            GoalKind::MoveCargo {
                commodity: CommodityKind::Grain,
                destination: entity(22),
            },
            GoalKindDiscriminant::MoveCargo,
        ),
        (
            GoalKind::LootCorpse { corpse: entity(23) },
            GoalKindDiscriminant::LootCorpse,
        ),
        (
            GoalKind::BuryCorpse {
                corpse: entity(24),
                burial_site: entity(25),
            },
            GoalKindDiscriminant::BuryCorpse,
        ),
        (
            GoalKind::FulfillBounty { bounty: entity(26) },
            GoalKindDiscriminant::FulfillBounty,
        ),
        (
            GoalKind::PostBounty {
                posting: posting(entity(27)),
                terms: bounty_terms(entity(28), entity(27)),
            },
            GoalKindDiscriminant::PostBounty,
        ),
        (
            GoalKind::PostNotice {
                posting: posting(entity(29)),
                topic: NoticeTopic::ThreatWarning { place: entity(30) },
            },
            GoalKindDiscriminant::PostNotice,
        ),
        (
            GoalKind::ShareBelief {
                listener: entity(31),
                topic: TellTopic::EntityBelief {
                    subject: entity(32),
                },
                communication_class: CommunicationClass::Testimony,
            },
            GoalKindDiscriminant::ShareBelief,
        ),
        (
            GoalKind::AskWitness {
                witness: entity(33),
                topic: TellTopic::EntityBelief {
                    subject: entity(34),
                },
            },
            GoalKindDiscriminant::AskWitness,
        ),
        (
            GoalKind::ClaimOffice { office: entity(35) },
            GoalKindDiscriminant::ClaimOffice,
        ),
        (
            GoalKind::SupportCandidateForOffice {
                office: entity(36),
                candidate: entity(37),
            },
            GoalKindDiscriminant::SupportCandidateForOffice,
        ),
        (
            GoalKind::InvestigateViolation {
                violation_id: ViolationId(6),
                place: entity(38),
            },
            GoalKindDiscriminant::InvestigateViolation,
        ),
        (
            GoalKind::Patrol { place: entity(39) },
            GoalKindDiscriminant::Patrol,
        ),
        (
            GoalKind::ExploreLocation {
                target_place: entity(40),
                motivating_need: ExplorationMotivation::Proactive,
                hypothesis: HypothesisKind::Proactive,
            },
            GoalKindDiscriminant::ExploreLocation,
        ),
        (
            GoalKind::StealItem {
                target_item: entity(41),
            },
            GoalKindDiscriminant::StealItem,
        ),
        (
            GoalKind::Accuse {
                crime_register: entity(42),
                accused: entity(43),
                violation_id: ViolationId(7),
            },
            GoalKindDiscriminant::Accuse,
        ),
        (
            GoalKind::PunishAccused {
                office: entity(44),
                accused: entity(45),
                accusation_entry: RecordEntryId(1),
                punishment: PunishmentKind::Fine {
                    commodity: CommodityKind::Coin,
                    amount: Quantity(2),
                },
            },
            GoalKindDiscriminant::PunishAccused,
        ),
    ]
}

#[test]
fn motive_source_round_trip_covers_all_variants() {
    for (source, expected) in motive_source_examples() {
        assert_eq!(source.discriminant(), expected);
        assert_eq!(MotiveSourceDiscriminant::from(&source), expected);
    }
}

#[test]
fn goal_kind_round_trip_covers_all_variants() {
    for (kind, expected) in goal_kind_examples() {
        assert_eq!(kind.discriminant(), expected);
        assert_eq!(GoalKindDiscriminant::from(&kind), expected);
    }
}

#[test]
fn goal_kind_all_constant_is_exhaustive_and_unique() {
    let examples = goal_kind_examples();
    let expected: BTreeSet<_> = examples
        .iter()
        .map(|(_, discriminant)| *discriminant)
        .collect();
    let all: BTreeSet<_> = GoalKindDiscriminant::ALL.iter().copied().collect();

    assert_eq!(GoalKindDiscriminant::ALL.len(), examples.len());
    assert_eq!(all.len(), GoalKindDiscriminant::ALL.len());
    assert_eq!(all, expected);
}

#[test]
fn method_schema_id_satisfies_key_and_serde_bounds() {
    let mut ids = BTreeSet::new();
    ids.insert(MethodSchemaId(7));

    let bytes = bincode::serialize(&ids).unwrap();
    let decoded: BTreeSet<MethodSchemaId> = bincode::deserialize(&bytes).unwrap();

    assert_eq!(decoded, ids);
}
