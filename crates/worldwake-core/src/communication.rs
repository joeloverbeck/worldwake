//! Communication-type classification and per-agent acceptance profiles.

use crate::{
    current_institutional_belief_topics, AgentBeliefStore, Component,
    InstitutionalKnowledgeSource, PerceptionSource, Permille, SocialObservationDetail, TellTopic,
};
use serde::{Deserialize, Serialize};

/// Classification of social communication by urgency and trust model.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CommunicationClass {
    Alarm,
    Testimony,
    Gossip,
}

/// Per-agent parameters controlling communication acceptance by class.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommunicationProfile {
    pub alarm_acceptance: Permille,
    pub testimony_acceptance: Permille,
    pub gossip_acceptance: Permille,
}

impl Default for CommunicationProfile {
    fn default() -> Self {
        Self {
            alarm_acceptance: Permille::new_unchecked(950),
            testimony_acceptance: Permille::new_unchecked(800),
            gossip_acceptance: Permille::new_unchecked(600),
        }
    }
}

impl Component for CommunicationProfile {}

/// Classify a Tell topic by urgency and trust model.
#[must_use]
pub fn classify_communication(
    topic: &TellTopic,
    speaker_beliefs: &AgentBeliefStore,
) -> CommunicationClass {
    match *topic {
        TellTopic::SocialObservation { observation } => match observation.detail {
            SocialObservationDetail::WitnessedConflict { .. } => CommunicationClass::Alarm,
            SocialObservationDetail::SuspectedTheft { .. }
            | SocialObservationDetail::WitnessedAbsence { .. } => CommunicationClass::Testimony,
            SocialObservationDetail::WitnessedCooperation { .. }
            | SocialObservationDetail::WitnessedObligation { .. }
            | SocialObservationDetail::WitnessedTelling { .. }
            | SocialObservationDetail::CoPresence { .. } => CommunicationClass::Gossip,
        },
        TellTopic::EntityBelief { subject } => speaker_beliefs
            .get_entity(&subject)
            .map_or(CommunicationClass::Gossip, |belief| {
                if belief.alive {
                    match belief.source {
                        PerceptionSource::DirectObservation | PerceptionSource::Report { .. } => {
                            CommunicationClass::Testimony
                        }
                        PerceptionSource::Rumor { .. } | PerceptionSource::Inference => {
                            CommunicationClass::Gossip
                        }
                    }
                } else {
                    CommunicationClass::Alarm
                }
            }),
        TellTopic::InstitutionalClaim { claim } => current_institutional_belief_topics(
            speaker_beliefs
                .institutional_beliefs
                .values()
                .flat_map(|beliefs| beliefs.iter().cloned()),
        )
        .into_iter()
        .find(|belief| belief.claim == claim)
        .map_or(CommunicationClass::Gossip, |belief| match belief.source {
            InstitutionalKnowledgeSource::DirectObservation
            | InstitutionalKnowledgeSource::WitnessedEvent
            | InstitutionalKnowledgeSource::RecordConsultation { .. }
            | InstitutionalKnowledgeSource::SelfDeclaration
            | InstitutionalKnowledgeSource::Report { .. } => CommunicationClass::Testimony,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_communication, CommunicationClass, CommunicationProfile};
    use crate::{
        traits::Component, AgentBeliefStore, BelievedEntityState, BelievedInstitutionalClaim,
        ControlSource, EntityId, EntityKind, InstitutionalBeliefKey, InstitutionalClaim,
        InstitutionalKnowledgeSource, PerceptionSource, Quantity, SocialObservation,
        SocialObservationDetail, TellTopic, Tick, Topology, World,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::BTreeMap;
    use std::fmt::Debug;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn base_belief_store() -> AgentBeliefStore {
        AgentBeliefStore::new()
    }

    #[test]
    fn communication_profile_component_bounds() {
        assert_component_bounds::<CommunicationProfile>();
        assert_value_bounds::<CommunicationProfile>();
    }

    #[test]
    fn classify_communication_marks_witnessed_conflict_as_alarm() {
        let topic = TellTopic::SocialObservation {
            observation: SocialObservation {
                detail: SocialObservationDetail::WitnessedConflict {
                    actor: entity(1),
                    target: entity(2),
                },
                place: entity(3),
                observed_tick: Tick(4),
                source: PerceptionSource::DirectObservation,
            },
        };

        assert_eq!(
            classify_communication(&topic, &base_belief_store()),
            CommunicationClass::Alarm
        );
    }

    #[test]
    fn classify_communication_marks_dead_entity_belief_as_alarm() {
        let subject = entity(7);
        let mut beliefs = base_belief_store();
        beliefs.update_entity(
            subject,
            BelievedEntityState {
                last_known_place: Some(entity(8)),
                last_known_inventory: BTreeMap::from([(crate::CommodityKind::Bread, Quantity(1))]),
                workstation_tag: None,
                resource_source: None,
                alive: false,
                wounds: Vec::new(),
                last_known_courage: None,
                believed_activity: None,
                believed_contention: None,
                observed_tick: Tick(10),
                source: PerceptionSource::DirectObservation,
            },
        );

        assert_eq!(
            classify_communication(&TellTopic::EntityBelief { subject }, &beliefs),
            CommunicationClass::Alarm
        );
    }

    #[test]
    fn classify_communication_marks_direct_entity_belief_as_testimony() {
        let subject = entity(11);
        let mut beliefs = base_belief_store();
        beliefs.update_entity(
            subject,
            BelievedEntityState {
                last_known_place: Some(entity(12)),
                last_known_inventory: BTreeMap::new(),
                workstation_tag: None,
                resource_source: None,
                alive: true,
                wounds: Vec::new(),
                last_known_courage: None,
                believed_activity: None,
                believed_contention: None,
                observed_tick: Tick(13),
                source: PerceptionSource::DirectObservation,
            },
        );

        assert_eq!(
            classify_communication(&TellTopic::EntityBelief { subject }, &beliefs),
            CommunicationClass::Testimony
        );
    }

    #[test]
    fn classify_communication_marks_rumor_entity_belief_as_gossip() {
        let subject = entity(14);
        let mut beliefs = base_belief_store();
        beliefs.update_entity(
            subject,
            BelievedEntityState {
                last_known_place: Some(entity(15)),
                last_known_inventory: BTreeMap::new(),
                workstation_tag: None,
                resource_source: None,
                alive: true,
                wounds: Vec::new(),
                last_known_courage: None,
                believed_activity: None,
                believed_contention: None,
                observed_tick: Tick(16),
                source: PerceptionSource::Rumor { chain_len: 2 },
            },
        );

        assert_eq!(
            classify_communication(&TellTopic::EntityBelief { subject }, &beliefs),
            CommunicationClass::Gossip
        );
    }

    #[test]
    fn classify_communication_marks_record_consultation_as_testimony() {
        let claim = InstitutionalClaim::OfficeHolder {
            office: entity(20),
            holder: Some(entity(21)),
            effective_tick: Tick(22),
        };
        let mut beliefs = base_belief_store();
        beliefs.institutional_beliefs.insert(
            InstitutionalBeliefKey::OfficeHolderOf { office: entity(20) },
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record: entity(23),
                    entry_id: crate::RecordEntryId(4),
                },
                learned_tick: Tick(24),
                learned_at: Some(entity(25)),
            }],
        );

        assert_eq!(
            classify_communication(&TellTopic::InstitutionalClaim { claim }, &beliefs),
            CommunicationClass::Testimony
        );
    }

    #[test]
    fn classify_communication_marks_copresence_as_gossip() {
        let topic = TellTopic::SocialObservation {
            observation: SocialObservation {
                detail: SocialObservationDetail::CoPresence { other: entity(30) },
                place: entity(31),
                observed_tick: Tick(32),
                source: PerceptionSource::DirectObservation,
            },
        };

        assert_eq!(
            classify_communication(&topic, &base_belief_store()),
            CommunicationClass::Gossip
        );
    }

    #[test]
    fn communication_profile_default_values_match_spec() {
        let profile = CommunicationProfile::default();

        assert_eq!(profile.alarm_acceptance, crate::Permille::new(950).unwrap());
        assert_eq!(profile.testimony_acceptance, crate::Permille::new(800).unwrap());
        assert_eq!(profile.gossip_acceptance, crate::Permille::new(600).unwrap());
    }

    #[test]
    fn communication_profile_roundtrips_through_bincode() {
        let profile = CommunicationProfile {
            alarm_acceptance: crate::Permille::new(975).unwrap(),
            testimony_acceptance: crate::Permille::new(825).unwrap(),
            gossip_acceptance: crate::Permille::new(525).unwrap(),
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: CommunicationProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn communication_profile_registers_for_agents() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Town Crier", ControlSource::Ai, Tick(1))
            .unwrap();
        let profile = CommunicationProfile {
            gossip_acceptance: crate::Permille::new(450).unwrap(),
            ..CommunicationProfile::default()
        };

        assert_eq!(
            world.remove_component_communication_profile(agent).unwrap(),
            Some(CommunicationProfile::default())
        );
        world
            .insert_component_communication_profile(agent, profile.clone())
            .unwrap();

        assert_eq!(world.get_component_communication_profile(agent), Some(&profile));
        assert_eq!(
            world.entities_with_communication_profile().collect::<Vec<_>>(),
            vec![agent]
        );
        assert_eq!(
            world.query_communication_profile().collect::<Vec<_>>(),
            vec![(agent, &profile)]
        );
        assert_eq!(world.count_with_communication_profile(), 1);
        assert_eq!(world.entity_kind(agent), Some(EntityKind::Agent));
    }
}
