use worldwake_core::{
    institutional_knowledge_chain_len, social_observation_is_relayable,
    BelievedEntityState, BelievedInstitutionalClaim, EntityId, PerceptionSource,
    RecipientKnowledgeStatus, SocialObservation, TellTopic, Tick,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TellTopicOmissionReason {
    DirectlyObservableByListener,
    ListenerParticipatedInObservation,
    NonRelayableSocialObservation,
    ExceedsRelayDepth,
    SpeakerHasAlreadyToldCurrentBelief,
    TruncatedByCandidateLimit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TellTopicOmission {
    pub topic: TellTopic,
    pub reason: TellTopicOmissionReason,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TellTopicSelection {
    pub selected: Vec<TellTopic>,
    pub omitted: Vec<TellTopicOmission>,
}

#[must_use]
pub fn belief_chain_len(source: PerceptionSource) -> u8 {
    match source {
        PerceptionSource::DirectObservation | PerceptionSource::Inference => 0,
        PerceptionSource::Report { chain_len, .. } | PerceptionSource::Rumor { chain_len } => {
            chain_len
        }
    }
}

fn tell_topic_priority(topic: &TellTopic) -> u8 {
    match topic {
        TellTopic::InstitutionalClaim { .. } => 0,
        TellTopic::SocialObservation { .. } => 1,
        TellTopic::EntityBelief { .. } => 2,
    }
}

fn institutional_claim_priority(claim: &worldwake_core::InstitutionalClaim) -> u8 {
    match claim {
        worldwake_core::InstitutionalClaim::ForceControl { .. } => 0,
        worldwake_core::InstitutionalClaim::OfficeHolder { .. } => 1,
        worldwake_core::InstitutionalClaim::SupportDeclaration { .. } => 2,
        worldwake_core::InstitutionalClaim::FactionMembership { .. } => 3,
        worldwake_core::InstitutionalClaim::Accusation { .. } => 4,
        worldwake_core::InstitutionalClaim::Verdict { .. } => 5,
    }
}

fn compare_tell_topics(left: &TellTopic, right: &TellTopic) -> std::cmp::Ordering {
    tell_topic_priority(left)
        .cmp(&tell_topic_priority(right))
        .then_with(|| match (left, right) {
            (
                TellTopic::InstitutionalClaim { claim: left_claim },
                TellTopic::InstitutionalClaim {
                    claim: right_claim,
                },
            ) => institutional_claim_priority(left_claim)
                .cmp(&institutional_claim_priority(right_claim))
                .then_with(|| left_claim.cmp(right_claim)),
            _ => left.cmp(right),
        })
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
            .then_with(|| compare_tell_topics(left_topic, right_topic))
    });
    topics.truncate(usize::from(max_tell_candidates));
    topics.into_iter().map(|(_, topic)| topic).collect()
}

#[must_use]
pub fn listener_aware_relayable_tell_topics(
    entity_beliefs: impl IntoIterator<Item = (EntityId, BelievedEntityState)>,
    social_observations: impl IntoIterator<Item = SocialObservation>,
    institutional_beliefs: impl IntoIterator<Item = BelievedInstitutionalClaim>,
    max_relay_chain_len: u8,
    max_tell_candidates: u8,
    recipient_knowledge_status: impl FnMut(&TellTopic) -> RecipientKnowledgeStatus,
) -> Vec<TellTopic> {
    listener_aware_tell_topic_selection(
        entity_beliefs,
        social_observations,
        institutional_beliefs,
        max_relay_chain_len,
        max_tell_candidates,
        recipient_knowledge_status,
    )
    .selected
}

#[must_use]
pub fn listener_aware_tell_topic_selection(
    entity_beliefs: impl IntoIterator<Item = (EntityId, BelievedEntityState)>,
    social_observations: impl IntoIterator<Item = SocialObservation>,
    institutional_beliefs: impl IntoIterator<Item = BelievedInstitutionalClaim>,
    max_relay_chain_len: u8,
    max_tell_candidates: u8,
    mut recipient_knowledge_status: impl FnMut(&TellTopic) -> RecipientKnowledgeStatus,
) -> TellTopicSelection {
    let mut eligible = Vec::new();
    let mut omitted = Vec::new();

    for (subject, belief) in entity_beliefs {
        let topic = TellTopic::EntityBelief { subject };
        let chain_len = belief_chain_len(belief.source);
        if chain_len > max_relay_chain_len {
            omitted.push(TellTopicOmission {
                topic,
                reason: TellTopicOmissionReason::ExceedsRelayDepth,
            });
            continue;
        }
        if recipient_knowledge_status(&topic)
            == RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
        {
            omitted.push(TellTopicOmission {
                topic,
                reason: TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief,
            });
            continue;
        }
        eligible.push((belief.observed_tick, topic));
    }

    for observation in social_observations {
        let topic = TellTopic::SocialObservation { observation };
        if !social_observation_is_relayable(&observation) {
            omitted.push(TellTopicOmission {
                topic,
                reason: TellTopicOmissionReason::NonRelayableSocialObservation,
            });
            continue;
        }
        let chain_len = belief_chain_len(observation.source);
        if chain_len > max_relay_chain_len {
            omitted.push(TellTopicOmission {
                topic,
                reason: TellTopicOmissionReason::ExceedsRelayDepth,
            });
            continue;
        }
        if recipient_knowledge_status(&topic)
            == RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
        {
            omitted.push(TellTopicOmission {
                topic,
                reason: TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief,
            });
            continue;
        }
        eligible.push((observation.observed_tick, topic));
    }

    for belief in institutional_beliefs {
        let topic = TellTopic::InstitutionalClaim { claim: belief.claim };
        let chain_len = institutional_knowledge_chain_len(belief.source);
        if chain_len > max_relay_chain_len {
            omitted.push(TellTopicOmission {
                topic,
                reason: TellTopicOmissionReason::ExceedsRelayDepth,
            });
            continue;
        }
        if recipient_knowledge_status(&topic)
            == RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
        {
            omitted.push(TellTopicOmission {
                topic,
                reason: TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief,
            });
            continue;
        }
        eligible.push((belief.learned_tick, topic));
    }

    eligible.sort_unstable_by(|(left_tick, left_topic), (right_tick, right_topic)| {
        right_tick
            .cmp(left_tick)
            .then_with(|| compare_tell_topics(left_topic, right_topic))
    });

    let keep_len = usize::from(max_tell_candidates);
    let mut selected = Vec::with_capacity(eligible.len().min(keep_len));
    for (idx, (_, topic)) in eligible.into_iter().enumerate() {
        if idx < keep_len {
            selected.push(topic);
        } else {
            omitted.push(TellTopicOmission {
                topic,
                reason: TellTopicOmissionReason::TruncatedByCandidateLimit,
            });
        }
    }

    TellTopicSelection { selected, omitted }
}

#[cfg(test)]
mod tests {
    use super::{
        belief_chain_len, listener_aware_relayable_subjects, listener_aware_relayable_tell_topics,
        listener_aware_tell_topic_selection, relayable_social_subjects, TellTopicOmission,
        TellTopicOmissionReason,
    };
    use std::collections::BTreeMap;
    use worldwake_core::{
        BelievedEntityState, BelievedInstitutionalClaim, EntityId, InstitutionalClaim,
        InstitutionalKnowledgeSource, PerceptionSource, RecipientKnowledgeStatus,
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

    fn social_observation(
        observed_tick: u64,
        detail: SocialObservationDetail,
    ) -> SocialObservation {
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
            Vec::<BelievedInstitutionalClaim>::new(),
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

    #[test]
    fn listener_aware_tell_topic_selection_reports_relay_filtering_reasons() {
        let fresh = believed_state(9, PerceptionSource::DirectObservation);
        let too_deep = believed_state(8, PerceptionSource::Rumor { chain_len: 4 });
        let relayable = social_observation(
            7,
            SocialObservationDetail::WitnessedConflict {
                actor: entity(1),
                target: entity(2),
            },
        );
        let non_relayable = social_observation(
            6,
            SocialObservationDetail::WitnessedTelling {
                speaker: entity(3),
                listener: entity(4),
            },
        );
        let selection = listener_aware_tell_topic_selection(
            vec![(entity(10), fresh), (entity(11), too_deep)],
            vec![relayable, non_relayable],
            Vec::<BelievedInstitutionalClaim>::new(),
            2,
            5,
            |topic| match topic {
                TellTopic::EntityBelief { subject } if *subject == entity(10) => {
                    RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
                }
                _ => RecipientKnowledgeStatus::UnknownToSpeaker,
            },
        );

        assert_eq!(
            selection.selected,
            vec![TellTopic::SocialObservation {
                observation: relayable,
            }]
        );
        assert!(selection.omitted.contains(&TellTopicOmission {
            topic: TellTopic::EntityBelief {
                subject: entity(10)
            },
            reason: TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief,
        }));
        assert!(selection.omitted.contains(&TellTopicOmission {
            topic: TellTopic::EntityBelief {
                subject: entity(11)
            },
            reason: TellTopicOmissionReason::ExceedsRelayDepth,
        }));
        assert!(selection.omitted.contains(&TellTopicOmission {
            topic: TellTopic::SocialObservation {
                observation: non_relayable,
            },
            reason: TellTopicOmissionReason::NonRelayableSocialObservation,
        }));
    }

    #[test]
    fn listener_aware_tell_topic_selection_reports_truncation_after_sorting() {
        let selection = listener_aware_tell_topic_selection(
            vec![
                (
                    entity(10),
                    believed_state(3, PerceptionSource::DirectObservation),
                ),
                (
                    entity(11),
                    believed_state(9, PerceptionSource::DirectObservation),
                ),
            ],
            vec![social_observation(
                8,
                SocialObservationDetail::WitnessedConflict {
                    actor: entity(1),
                    target: entity(2),
                },
            )],
            Vec::<BelievedInstitutionalClaim>::new(),
            2,
            2,
            |_| RecipientKnowledgeStatus::UnknownToSpeaker,
        );

        assert_eq!(selection.selected.len(), 2);
        assert!(selection.omitted.contains(&TellTopicOmission {
            topic: TellTopic::EntityBelief {
                subject: entity(10)
            },
            reason: TellTopicOmissionReason::TruncatedByCandidateLimit,
        }));
    }

    #[test]
    fn listener_aware_tell_topic_selection_prefers_fresher_topics_before_kind_priority() {
        let stale_claim = InstitutionalClaim::OfficeHolder {
            office: entity(50),
            holder: Some(entity(51)),
            effective_tick: Tick(4),
        };
        let stale_institutional = BelievedInstitutionalClaim {
            claim: stale_claim,
            source: InstitutionalKnowledgeSource::WitnessedEvent,
            learned_tick: Tick(4),
            learned_at: Some(entity(60)),
        };
        let fresh_entity = (
            entity(10),
            believed_state(9, PerceptionSource::DirectObservation),
        );

        let selection = listener_aware_tell_topic_selection(
            vec![fresh_entity],
            Vec::<SocialObservation>::new(),
            vec![stale_institutional],
            2,
            1,
            |_| RecipientKnowledgeStatus::UnknownToSpeaker,
        );

        assert_eq!(
            selection.selected,
            vec![TellTopic::EntityBelief { subject: entity(10) }]
        );
        assert!(selection.omitted.contains(&TellTopicOmission {
            topic: TellTopic::InstitutionalClaim {
                claim: stale_claim,
            },
            reason: TellTopicOmissionReason::TruncatedByCandidateLimit,
        }));
    }
}
