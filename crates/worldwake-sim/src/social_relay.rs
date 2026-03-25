use worldwake_core::{
    social_observation_is_relayable, BelievedEntityState, EntityId, PerceptionSource,
    RecipientKnowledgeStatus, SocialObservation, TellTopic, Tick,
};

#[must_use]
pub fn belief_chain_len(source: PerceptionSource) -> u8 {
    match source {
        PerceptionSource::DirectObservation | PerceptionSource::Inference => 0,
        PerceptionSource::Report { chain_len, .. } | PerceptionSource::Rumor { chain_len } => {
            chain_len
        }
    }
}

#[must_use]
pub fn relayable_social_subjects(
    beliefs: impl IntoIterator<Item = (EntityId, BelievedEntityState)>,
    max_relay_chain_len: u8,
    max_tell_candidates: u8,
) -> Vec<EntityId> {
    let mut subjects = beliefs
        .into_iter()
        .filter_map(|(subject, belief)| {
            (belief_chain_len(belief.source) <= max_relay_chain_len)
                .then_some((belief.observed_tick, subject))
        })
        .collect::<Vec<_>>();
    subjects.sort_unstable_by(|(left_tick, left_subject), (right_tick, right_subject)| {
        right_tick
            .cmp(left_tick)
            .then_with(|| left_subject.cmp(right_subject))
    });
    subjects.truncate(usize::from(max_tell_candidates));
    subjects.into_iter().map(|(_, subject)| subject).collect()
}

#[must_use]
pub fn listener_aware_relayable_subjects(
    beliefs: impl IntoIterator<Item = (EntityId, BelievedEntityState)>,
    max_relay_chain_len: u8,
    max_tell_candidates: u8,
    mut recipient_knowledge_status: impl FnMut(
        EntityId,
        &BelievedEntityState,
    ) -> RecipientKnowledgeStatus,
) -> Vec<EntityId> {
    relayable_social_subjects(
        beliefs.into_iter().filter(|(subject, belief)| {
            recipient_knowledge_status(*subject, belief)
                != RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
        }),
        max_relay_chain_len,
        max_tell_candidates,
    )
}

#[must_use]
pub fn relayable_tell_topics(
    topics: impl IntoIterator<Item = (TellTopic, Tick, u8)>,
    max_relay_chain_len: u8,
    max_tell_candidates: u8,
) -> Vec<TellTopic> {
    let mut topics = topics
        .into_iter()
        .filter_map(|(topic, observed_tick, chain_len)| {
            (chain_len <= max_relay_chain_len).then_some((observed_tick, topic))
        })
        .collect::<Vec<_>>();
    topics.sort_unstable_by(|(left_tick, left_topic), (right_tick, right_topic)| {
        right_tick
            .cmp(left_tick)
            .then_with(|| left_topic.cmp(right_topic))
    });
    topics.truncate(usize::from(max_tell_candidates));
    topics.into_iter().map(|(_, topic)| topic).collect()
}

#[must_use]
pub fn listener_aware_relayable_tell_topics(
    entity_beliefs: impl IntoIterator<Item = (EntityId, BelievedEntityState)>,
    social_observations: impl IntoIterator<Item = SocialObservation>,
    max_relay_chain_len: u8,
    max_tell_candidates: u8,
    mut recipient_knowledge_status: impl FnMut(&TellTopic) -> RecipientKnowledgeStatus,
) -> Vec<TellTopic> {
    relayable_tell_topics(
        entity_beliefs
            .into_iter()
            .map(|(subject, belief)| {
                (
                    TellTopic::EntityBelief { subject },
                    belief.observed_tick,
                    belief_chain_len(belief.source),
                )
            })
            .chain(social_observations.into_iter().map(|observation| {
                (
                    TellTopic::SocialObservation { observation },
                    observation.observed_tick,
                    belief_chain_len(observation.source),
                )
            }))
            .filter(|(topic, _, _)| match topic {
                TellTopic::EntityBelief { .. } => true,
                TellTopic::SocialObservation { observation } => {
                    social_observation_is_relayable(observation)
                }
            })
            .filter(|(topic, _, _)| {
                recipient_knowledge_status(topic)
                    != RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
            }),
        max_relay_chain_len,
        max_tell_candidates,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        belief_chain_len, listener_aware_relayable_subjects, listener_aware_relayable_tell_topics,
        relayable_social_subjects,
    };
    use std::collections::BTreeMap;
    use worldwake_core::{
        BelievedEntityState, EntityId, PerceptionSource, RecipientKnowledgeStatus,
        SocialObservation, SocialObservationDetail, TellTopic, Tick,
    };

    fn entity(id: u64) -> EntityId {
        EntityId {
            slot: id as u32,
            generation: 0,
        }
    }

    fn believed_state(observed_tick: u64, source: PerceptionSource) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: None,
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            observed_tick: Tick(observed_tick),
            source,
        }
    }

    fn social_observation(observed_tick: u64, detail: SocialObservationDetail) -> SocialObservation {
        SocialObservation {
            detail,
            place: entity(40),
            observed_tick: Tick(observed_tick),
            source: PerceptionSource::DirectObservation,
        }
    }

    #[test]
    fn chain_length_maps_sources_to_expected_depth() {
        assert_eq!(belief_chain_len(PerceptionSource::DirectObservation), 0);
        assert_eq!(belief_chain_len(PerceptionSource::Inference), 0);
        assert_eq!(
            belief_chain_len(PerceptionSource::Report {
                from: entity(1),
                chain_len: 2,
            }),
            2
        );
        assert_eq!(
            belief_chain_len(PerceptionSource::Rumor { chain_len: 3 }),
            3
        );
    }

    #[test]
    fn relayable_subjects_filter_sort_and_truncate() {
        let subjects = relayable_social_subjects(
            vec![
                (
                    entity(10),
                    believed_state(3, PerceptionSource::DirectObservation),
                ),
                (
                    entity(11),
                    believed_state(
                        9,
                        PerceptionSource::Report {
                            from: entity(80),
                            chain_len: 2,
                        },
                    ),
                ),
                (entity(12), believed_state(9, PerceptionSource::Inference)),
                (
                    entity(13),
                    believed_state(7, PerceptionSource::Rumor { chain_len: 3 }),
                ),
                (
                    entity(14),
                    believed_state(5, PerceptionSource::Rumor { chain_len: 1 }),
                ),
            ],
            2,
            3,
        );

        assert_eq!(subjects, vec![entity(11), entity(12), entity(14)]);
    }

    #[test]
    fn relayable_subjects_allow_zero_candidate_limit() {
        let subjects = relayable_social_subjects(
            vec![(
                entity(10),
                believed_state(3, PerceptionSource::DirectObservation),
            )],
            3,
            0,
        );

        assert!(subjects.is_empty());
    }

    #[test]
    fn listener_aware_relayable_subjects_skip_already_told_current_beliefs() {
        let subjects = listener_aware_relayable_subjects(
            vec![
                (
                    entity(10),
                    believed_state(9, PerceptionSource::DirectObservation),
                ),
                (
                    entity(11),
                    believed_state(7, PerceptionSource::DirectObservation),
                ),
            ],
            3,
            3,
            |subject, _| match subject {
                s if s == entity(10) => {
                    RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
                }
                _ => RecipientKnowledgeStatus::UnknownToSpeaker,
            },
        );

        assert_eq!(subjects, vec![entity(11)]);
    }

    #[test]
    fn listener_aware_relayable_subjects_filter_before_truncation() {
        let subjects = listener_aware_relayable_subjects(
            vec![
                (
                    entity(10),
                    believed_state(10, PerceptionSource::DirectObservation),
                ),
                (
                    entity(11),
                    believed_state(8, PerceptionSource::DirectObservation),
                ),
            ],
            3,
            1,
            |subject, _| match subject {
                s if s == entity(10) => {
                    RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
                }
                _ => RecipientKnowledgeStatus::UnknownToSpeaker,
            },
        );

        assert_eq!(subjects, vec![entity(11)]);
    }

    #[test]
    fn listener_aware_relayable_subjects_reinclude_stale_or_expired_tells() {
        let subjects = listener_aware_relayable_subjects(
            vec![
                (
                    entity(10),
                    believed_state(10, PerceptionSource::DirectObservation),
                ),
                (
                    entity(11),
                    believed_state(8, PerceptionSource::DirectObservation),
                ),
            ],
            3,
            2,
            |subject, _| match subject {
                s if s == entity(10) => RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief,
                s if s == entity(11) => {
                    RecipientKnowledgeStatus::SpeakerPreviouslyToldButMemoryExpired
                }
                _ => RecipientKnowledgeStatus::UnknownToSpeaker,
            },
        );

        assert_eq!(subjects, vec![entity(10), entity(11)]);
    }

    #[test]
    fn listener_aware_relayable_tell_topics_exclude_witnessed_telling_observations() {
        let relayable = social_observation(
            9,
            SocialObservationDetail::WitnessedConflict {
                actor: entity(1),
                target: entity(2),
            },
        );
        let witnessed_telling = social_observation(
            10,
            SocialObservationDetail::WitnessedTelling {
                speaker: entity(3),
                listener: entity(4),
            },
        );

        let topics = listener_aware_relayable_tell_topics(
            Vec::<(EntityId, BelievedEntityState)>::new(),
            vec![relayable, witnessed_telling],
            2,
            5,
            |_| RecipientKnowledgeStatus::UnknownToSpeaker,
        );

        assert_eq!(
            topics,
            vec![TellTopic::SocialObservation {
                observation: relayable,
            }]
        );
    }
}
