use serde::{Deserialize, Serialize};

use crate::{
    EntityBeliefAspect, InstitutionalClaim, SocialObservation, SocialObservationDetail, TellTopic,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum TopicScope {
    RouteHazard,
    ResourceAvailability,
    OfficeHolder,
    AccusationCredibility,
    BountyValidity,
    PriceLevel,
    EntityWhereabouts,
    GeneralFact,
}

#[must_use]
pub fn belief_topic_to_topic_scope(topic: &TellTopic) -> TopicScope {
    match topic {
        TellTopic::EntityBelief { .. } => TopicScope::GeneralFact,
        TellTopic::SocialObservation { observation } => {
            social_observation_to_topic_scope(observation)
        }
        TellTopic::InstitutionalClaim { claim } => institutional_claim_to_topic_scope(claim),
    }
}

#[must_use]
pub fn entity_aspect_to_topic_scope(aspect: &EntityBeliefAspect) -> TopicScope {
    match aspect {
        EntityBeliefAspect::Location
        | EntityBeliefAspect::Holder
        | EntityBeliefAspect::Alive
        | EntityBeliefAspect::Wounded
        | EntityBeliefAspect::Activity
        | EntityBeliefAspect::Courage => TopicScope::EntityWhereabouts,
        EntityBeliefAspect::Inventory(_)
        | EntityBeliefAspect::WorkstationPresent
        | EntityBeliefAspect::ResourceAvailable(_)
        | EntityBeliefAspect::ContentionState
        | EntityBeliefAspect::WashBasinState => TopicScope::ResourceAvailability,
        EntityBeliefAspect::Evidence => TopicScope::AccusationCredibility,
        EntityBeliefAspect::Owner | EntityBeliefAspect::Artifact => TopicScope::GeneralFact,
    }
}

fn social_observation_to_topic_scope(observation: &SocialObservation) -> TopicScope {
    match observation.detail {
        SocialObservationDetail::WitnessedConflict { .. } => TopicScope::RouteHazard,
        SocialObservationDetail::WitnessedAbsence { .. }
        | SocialObservationDetail::SuspectedTheft { .. } => TopicScope::AccusationCredibility,
        SocialObservationDetail::WitnessedCooperation { .. }
        | SocialObservationDetail::WitnessedObligation { .. }
        | SocialObservationDetail::WitnessedTelling { .. }
        | SocialObservationDetail::CoPresence { .. } => TopicScope::GeneralFact,
    }
}

fn institutional_claim_to_topic_scope(claim: &InstitutionalClaim) -> TopicScope {
    match claim {
        InstitutionalClaim::OfficeHolder { .. }
        | InstitutionalClaim::SupportDeclaration { .. }
        | InstitutionalClaim::ForceControl { .. } => TopicScope::OfficeHolder,
        InstitutionalClaim::Accusation { .. }
        | InstitutionalClaim::Verdict { .. }
        | InstitutionalClaim::ArtifactCredibilityRefutation { .. } => {
            TopicScope::AccusationCredibility
        }
        InstitutionalClaim::MissingPersonStatus { .. } => TopicScope::EntityWhereabouts,
        InstitutionalClaim::FactionMembership { .. }
        | InstitutionalClaim::FactionRallyPoint { .. } => TopicScope::GeneralFact,
    }
}

#[cfg(test)]
mod tests {
    use super::{TopicScope, belief_topic_to_topic_scope, entity_aspect_to_topic_scope};
    use crate::{
        CommodityKind, EntityBeliefAspect, EntityId, InstitutionalClaim, PerceptionSource,
        PunishmentKind, Quantity, SocialObservation, SocialObservationDetail, TellTopic,
        TheftFacts, Tick, ViolationId, institutional::MissingPersonReportStatus,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;
    use std::hash::Hash;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn theft_facts() -> TheftFacts {
        TheftFacts {
            missing_entity: entity(20),
            expected_place: entity(21),
            commodity: CommodityKind::Bread,
            quantity: Quantity(2),
        }
    }

    fn observation(detail: SocialObservationDetail) -> SocialObservation {
        SocialObservation {
            detail,
            place: entity(1),
            observed_tick: Tick(7),
            source: PerceptionSource::DirectObservation,
        }
    }

    fn assert_bounds<T>()
    where
        T: Copy + Clone + Debug + Eq + Ord + Hash + Serialize + DeserializeOwned,
    {
    }

    #[test]
    fn topic_scope_satisfies_required_bounds() {
        assert_bounds::<TopicScope>();
    }

    #[test]
    fn entity_belief_aspects_map_to_topic_scopes() {
        let cases = [
            (EntityBeliefAspect::Location, TopicScope::EntityWhereabouts),
            (EntityBeliefAspect::Owner, TopicScope::GeneralFact),
            (EntityBeliefAspect::Holder, TopicScope::EntityWhereabouts),
            (
                EntityBeliefAspect::Inventory(CommodityKind::Bread),
                TopicScope::ResourceAvailability,
            ),
            (EntityBeliefAspect::Alive, TopicScope::EntityWhereabouts),
            (EntityBeliefAspect::Wounded, TopicScope::EntityWhereabouts),
            (EntityBeliefAspect::Activity, TopicScope::EntityWhereabouts),
            (
                EntityBeliefAspect::WorkstationPresent,
                TopicScope::ResourceAvailability,
            ),
            (
                EntityBeliefAspect::ResourceAvailable(CommodityKind::Water),
                TopicScope::ResourceAvailability,
            ),
            (
                EntityBeliefAspect::ContentionState,
                TopicScope::ResourceAvailability,
            ),
            (
                EntityBeliefAspect::WashBasinState,
                TopicScope::ResourceAvailability,
            ),
            (EntityBeliefAspect::Artifact, TopicScope::GeneralFact),
            (EntityBeliefAspect::Courage, TopicScope::EntityWhereabouts),
            (
                EntityBeliefAspect::Evidence,
                TopicScope::AccusationCredibility,
            ),
        ];

        for (aspect, expected) in cases {
            assert_eq!(entity_aspect_to_topic_scope(&aspect), expected);
        }
    }

    #[test]
    fn social_observations_map_to_topic_scopes() {
        let cases = [
            (
                observation(SocialObservationDetail::WitnessedConflict {
                    actor: entity(2),
                    target: entity(3),
                }),
                TopicScope::RouteHazard,
            ),
            (
                observation(SocialObservationDetail::WitnessedAbsence {
                    missing_entity: entity(4),
                    expected_place: entity(5),
                }),
                TopicScope::AccusationCredibility,
            ),
            (
                observation(SocialObservationDetail::SuspectedTheft {
                    theft: theft_facts(),
                    suspect: Some(entity(6)),
                }),
                TopicScope::AccusationCredibility,
            ),
            (
                observation(SocialObservationDetail::WitnessedCooperation {
                    actor: entity(7),
                    counterpart: entity(8),
                }),
                TopicScope::GeneralFact,
            ),
            (
                observation(SocialObservationDetail::WitnessedObligation {
                    actor: entity(9),
                    target: entity(10),
                }),
                TopicScope::GeneralFact,
            ),
            (
                observation(SocialObservationDetail::WitnessedTelling {
                    speaker: entity(11),
                    listener: entity(12),
                }),
                TopicScope::GeneralFact,
            ),
            (
                observation(SocialObservationDetail::CoPresence { other: entity(13) }),
                TopicScope::GeneralFact,
            ),
        ];

        for (observation, expected) in cases {
            assert_eq!(
                belief_topic_to_topic_scope(&TellTopic::SocialObservation { observation }),
                expected
            );
        }
    }

    #[test]
    fn institutional_claims_map_to_topic_scopes() {
        let cases = [
            (
                InstitutionalClaim::OfficeHolder {
                    office: entity(30),
                    holder: Some(entity(31)),
                    effective_tick: Tick(1),
                },
                TopicScope::OfficeHolder,
            ),
            (
                InstitutionalClaim::SupportDeclaration {
                    office: entity(32),
                    supporter: entity(33),
                    candidate: Some(entity(34)),
                    effective_tick: Tick(2),
                },
                TopicScope::OfficeHolder,
            ),
            (
                InstitutionalClaim::ForceControl {
                    office: entity(35),
                    controller: Some(entity(36)),
                    contested: false,
                    effective_tick: Tick(3),
                },
                TopicScope::OfficeHolder,
            ),
            (
                InstitutionalClaim::Accusation {
                    accuser: entity(37),
                    accused: entity(38),
                    violation_id: ViolationId(1),
                    theft: theft_facts(),
                    effective_tick: Tick(4),
                },
                TopicScope::AccusationCredibility,
            ),
            (
                InstitutionalClaim::Verdict {
                    accused: entity(39),
                    violation_id: ViolationId(2),
                    punishment: PunishmentKind::Fine {
                        commodity: CommodityKind::Coin,
                        amount: Quantity(1),
                    },
                    effective_tick: Tick(5),
                },
                TopicScope::AccusationCredibility,
            ),
            (
                InstitutionalClaim::ArtifactCredibilityRefutation {
                    artifact: entity(40),
                    evidence: entity(41),
                    effective_tick: Tick(6),
                },
                TopicScope::AccusationCredibility,
            ),
            (
                InstitutionalClaim::MissingPersonStatus {
                    subject: entity(42),
                    reporter: entity(43),
                    status: MissingPersonReportStatus::FoundSafe {
                        at_place: entity(44),
                    },
                    effective_tick: Tick(7),
                },
                TopicScope::EntityWhereabouts,
            ),
            (
                InstitutionalClaim::FactionMembership {
                    faction: entity(45),
                    member: entity(46),
                    active: true,
                    effective_tick: Tick(8),
                },
                TopicScope::GeneralFact,
            ),
            (
                InstitutionalClaim::FactionRallyPoint {
                    faction: entity(47),
                    rally_place: Some(entity(48)),
                    effective_tick: Tick(9),
                },
                TopicScope::GeneralFact,
            ),
        ];

        for (claim, expected) in cases {
            assert_eq!(
                belief_topic_to_topic_scope(&TellTopic::InstitutionalClaim { claim }),
                expected
            );
        }
    }

    #[test]
    fn entity_tell_topic_without_aspect_maps_to_general_fact() {
        assert_eq!(
            belief_topic_to_topic_scope(&TellTopic::EntityBelief {
                subject: entity(50)
            }),
            TopicScope::GeneralFact
        );
    }
}
