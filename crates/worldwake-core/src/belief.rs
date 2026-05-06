//! Authoritative belief and perception state for E14.

use crate::{
    ActionDomain, BeliefClaimKey, BelievedInstitutionalClaim, ClaimId, ClaimValue, CommodityKind,
    Component, EntityBeliefAspect, EntityBeliefClaim, EntityId, EntityKind, EvidenceKind,
    HomeostaticNeedId, HomeostaticNeeds, InstitutionalBeliefKey, InstitutionalBeliefRead,
    InstitutionalClaim, InstitutionalKnowledgeSource, Permille, Quantity, ResourceSource,
    TheftFacts, Tick, WashBasinState, WorkstationTag, World, Wound,
    institutional::MissingPersonReportStatus,
    social_artifact::{ArtifactKind, ArtifactState, BountyTarget, NoticeTopic},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum InstitutionalTellTopicKey {
    OfficeHolder {
        office: EntityId,
    },
    ForceControl {
        office: EntityId,
    },
    FactionMembership {
        faction: EntityId,
        member: EntityId,
    },
    FactionRallyPoint {
        faction: EntityId,
    },
    SupportDeclaration {
        supporter: EntityId,
        office: EntityId,
    },
    CrimeCase {
        accused: EntityId,
        violation_id: crate::ViolationId,
    },
    MissingPersonStatus {
        subject: EntityId,
    },
}

/// Per-agent subjective view of observed entities and social evidence.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentBeliefStore {
    #[serde(default)]
    pub entity_claims: BTreeMap<EntityId, Vec<EntityBeliefClaim>>,
    #[serde(default)]
    pub next_claim_id: ClaimId,
    pub known_entities: BTreeMap<EntityId, BelievedEntityState>,
    pub social_observations: Vec<SocialObservation>,
    #[serde(default)]
    pub observation_omission_log: ObservationOmissionLog,
    pub told_beliefs: BTreeMap<TellMemoryKey, ToldBeliefMemory>,
    pub heard_beliefs: BTreeMap<TellMemoryKey, HeardBeliefMemory>,
    pub asked_witnesses: BTreeMap<AskWitnessMemoryKey, AskWitnessMemory>,
    #[serde(default)]
    pub place_visits: BTreeMap<EntityId, PlaceVisitRecord>,
    pub institutional_beliefs: BTreeMap<InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>>,
}

#[derive(Copy, Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum SaliencePolicy {
    #[default]
    PriorityWithNeedBoost,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum OmissionReason {
    OverBudget { budget: u8, candidates_seen: u16 },
    SalienceBelowFloor { policy: SaliencePolicy },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationOmission {
    pub omitted_entity: EntityId,
    pub reason: OmissionReason,
    pub observed_tick: Tick,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationOmissionLog {
    pub entries: VecDeque<ObservationOmission>,
}

/// Tracks an agent's visit history for a believed place.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlaceVisitRecord {
    pub ticks_present: u32,
    pub last_arrival_tick: Tick,
    pub visit_count: u16,
}

impl AgentBeliefStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_entity_claim(&mut self, claim: EntityBeliefClaim) {
        self.next_claim_id = ClaimId(claim.claim_id.0.saturating_add(1).max(self.next_claim_id.0));
        let claims = self.entity_claims.entry(claim.subject).or_default();
        let new_kind = source_kind(&claim.source);
        if claims.iter().any(|existing| {
            existing.aspect == claim.aspect
                && source_kind(&existing.source) == new_kind
                && claim_dominates(existing, &claim)
        }) {
            return;
        }
        if source_kind(&claim.source) == SourceKind::Direct {
            refute_conflicting_claims_for_direct_observation(claims, &claim);
        }
        claims.retain(|existing| {
            existing.aspect != claim.aspect
                || source_kind(&existing.source) != new_kind
                || !claim_dominates(&claim, existing)
                || existing.value != claim.value
        });
        claims.push(claim);
    }

    pub fn refute_entity_claims(
        &mut self,
        claim_key: BeliefClaimKey,
        refuted_at_tick: Tick,
        current_tick: Tick,
        policy: &BeliefConfidencePolicy,
    ) -> bool {
        let Some(claims) = self.entity_claims.get_mut(&claim_key.subject) else {
            return false;
        };
        let mut changed = false;
        for claim in claims.iter_mut() {
            if claim.aspect == claim_key.aspect && claim.refuted_at_tick.is_none() {
                claim.refuted_at_tick = Some(refuted_at_tick);
                changed = true;
            }
        }
        if changed {
            self.refresh_entity_summary_from_claims(claim_key.subject, current_tick, policy);
        }
        changed
    }

    pub fn update_entity(&mut self, id: EntityId, state: BelievedEntityState) {
        match self.known_entities.get(&id) {
            Some(existing)
                if existing.last_observed_tick().unwrap_or(Tick(0))
                    > state.last_observed_tick().unwrap_or(Tick(0)) => {}
            _ => {
                self.known_entities.insert(id, state);
            }
        }
    }

    pub fn import_entity_snapshot(
        &mut self,
        subject: EntityId,
        snapshot: &BelievedEntityState,
        current_tick: Tick,
        policy: &BeliefConfidencePolicy,
    ) {
        let prior = self.known_entities.get(&subject).cloned();
        if prior.as_ref().is_some_and(|existing| {
            existing.last_observed_tick().unwrap_or(Tick(0))
                > snapshot.last_observed_tick().unwrap_or(Tick(0))
        }) {
            return;
        }
        self.record_entity_snapshot_claims(
            subject,
            snapshot,
            prior.as_ref(),
            current_tick,
            snapshot.last_observed_tick(),
            0,
            policy,
        );
        if let Some(summary) = self.known_entities.get_mut(&subject) {
            summary.presentation_ticks = snapshot.presentation_ticks;
            summary.presentation_tick_count = snapshot.presentation_tick_count;
            if summary.believed_kind.is_none() {
                summary.believed_kind = snapshot.believed_kind;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_entity_snapshot_claims(
        &mut self,
        subject: EntityId,
        snapshot: &BelievedEntityState,
        prior_summary: Option<&BelievedEntityState>,
        current_tick: Tick,
        claimed_event_tick: Option<Tick>,
        observation_buffer_capacity: u8,
        policy: &BeliefConfidencePolicy,
    ) {
        let confidence = belief_confidence(&snapshot.source, 0, policy);
        for (aspect, value) in entity_claims_for_snapshot(snapshot, prior_summary) {
            self.record_entity_claim(EntityBeliefClaim {
                claim_id: self.next_claim_id,
                subject,
                aspect,
                value,
                source: snapshot.source,
                acquired_tick: current_tick,
                claimed_event_tick,
                confidence,
                refuted_at_tick: None,
            });
        }
        self.refresh_entity_summary_from_claims(subject, current_tick, policy);
        if snapshot.believed_kind.is_some()
            && let Some(summary) = self.known_entities.get_mut(&subject)
        {
            if summary.believed_kind.is_none() {
                summary.believed_kind = snapshot.believed_kind;
            }
            summary.push_presentation_tick(current_tick, observation_buffer_capacity);
        }
    }

    pub fn refresh_entity_summary_from_claims(
        &mut self,
        subject: EntityId,
        current_tick: Tick,
        policy: &BeliefConfidencePolicy,
    ) {
        let prior = self.known_entities.get(&subject);
        match self
            .entity_claims
            .get(&subject)
            .and_then(|claims| derive_entity_summary(claims, current_tick, policy))
        {
            Some(mut summary) => {
                preserve_believed_kind(prior, &mut summary);
                preserve_presentation_history(prior, &mut summary);
                self.known_entities.insert(subject, summary);
            }
            None => {
                self.known_entities.remove(&subject);
            }
        }
    }

    #[must_use]
    pub fn get_entity(&self, id: &EntityId) -> Option<&BelievedEntityState> {
        self.known_entities.get(id)
    }

    pub fn record_social_observation(&mut self, observation: SocialObservation) {
        if let Some(existing) = self
            .social_observations
            .iter_mut()
            .find(|existing| existing.detail == observation.detail)
        {
            if observation.observed_tick > existing.observed_tick {
                *existing = observation;
            }
            return;
        }

        self.social_observations.push(observation);
    }

    pub fn record_told_belief(&mut self, key: TellMemoryKey, memory: ToldBeliefMemory) {
        self.told_beliefs.insert(key, memory);
    }

    pub fn record_heard_belief(&mut self, key: TellMemoryKey, memory: HeardBeliefMemory) {
        self.heard_beliefs.insert(key, memory);
    }

    pub fn record_asked_witness(&mut self, key: AskWitnessMemoryKey, memory: AskWitnessMemory) {
        self.asked_witnesses.insert(key, memory);
    }

    pub fn record_place_visit(&mut self, place: EntityId, current_tick: Tick) -> bool {
        let record = match self.place_visits.entry(place) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(PlaceVisitRecord {
                    ticks_present: 0,
                    last_arrival_tick: current_tick,
                    visit_count: 1,
                });
                return true;
            }
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
        };

        if record.last_arrival_tick == current_tick {
            return false;
        }

        let expected_continuation_tick = record
            .last_arrival_tick
            .0
            .saturating_add(u64::from(record.ticks_present))
            .saturating_add(1);
        if current_tick.0 == expected_continuation_tick {
            record.ticks_present = record.ticks_present.saturating_add(1);
            return true;
        }

        record.ticks_present = 0;
        record.last_arrival_tick = current_tick;
        record.visit_count = record.visit_count.saturating_add(1);
        true
    }

    pub fn record_institutional_belief(
        &mut self,
        key: InstitutionalBeliefKey,
        belief: BelievedInstitutionalClaim,
        profile: &PerceptionProfile,
    ) {
        self.institutional_beliefs
            .entry(key)
            .or_default()
            .push(belief);
        self.enforce_institutional_capacity(profile);
    }

    pub fn replace_institutional_belief(
        &mut self,
        key: InstitutionalBeliefKey,
        belief: BelievedInstitutionalClaim,
        profile: &PerceptionProfile,
    ) {
        self.institutional_beliefs.insert(key, vec![belief]);
        self.enforce_institutional_capacity(profile);
    }

    pub fn prune_decayed_beliefs(
        &mut self,
        profile: &PerceptionProfile,
        current_tick: Tick,
        agent_needs: &HomeostaticNeeds,
    ) {
        let entity_activation_threshold = profile.entity_activation_threshold.value();
        self.social_observations.retain(|observation| {
            compute_activation(current_tick, &[observation.observed_tick], 1)
                >= entity_activation_threshold
        });

        let affected_entities = self.entity_claims.keys().copied().collect::<Vec<_>>();
        let claim_confidence_threshold = profile.claim_confidence_threshold.value();
        let mut changed_entities = Vec::new();
        for entity in &affected_entities {
            let retention_boost = self.known_entities.get(entity).map_or(0, |state| {
                state_salience_boost(
                    agent_needs,
                    state,
                    profile.need_salience_urgency_threshold,
                    profile.need_salience_boost,
                )
            });
            let Some(claims) = self.entity_claims.get_mut(entity) else {
                continue;
            };
            let len_before = claims.len();
            claims.retain(|claim| {
                let source_boost = if source_kind(&claim.source) == SourceKind::Direct {
                    retention_boost
                } else {
                    0
                };
                claim_is_refuted(claim)
                    || effective_claim_confidence(claim, current_tick, &profile.confidence_policy)
                        .saturating_add(source_boost)
                        >= claim_confidence_threshold
            });
            if claims.len() != len_before {
                changed_entities.push(*entity);
            }
        }
        self.entity_claims.retain(|_, claims| !claims.is_empty());
        if claim_confidence_threshold == 0 {
            for entity in affected_entities {
                self.refresh_entity_summary_from_claims(
                    entity,
                    current_tick,
                    &profile.confidence_policy,
                );
            }
        } else {
            for entity in changed_entities {
                self.refresh_entity_summary_from_claims(
                    entity,
                    current_tick,
                    &profile.confidence_policy,
                );
            }
        }

        self.known_entities.retain(|_, state| {
            let base = compute_activation(
                current_tick,
                &state.presentation_ticks,
                state.presentation_tick_count,
            );
            let boost = state_salience_boost(
                agent_needs,
                state,
                profile.need_salience_urgency_threshold,
                profile.need_salience_boost,
            );
            base.saturating_add(boost) >= entity_activation_threshold
        });
        self.entity_claims.retain(|entity, claims| {
            self.known_entities.contains_key(entity) || claims.iter().any(claim_is_refuted)
        });
    }

    pub fn enforce_conversation_memory(&mut self, profile: &TellProfile, current_tick: Tick) {
        self.told_beliefs.retain(|_, memory| {
            within_retention_window(
                memory.told_tick,
                current_tick,
                profile.conversation_memory_retention_ticks,
            )
        });
        self.heard_beliefs.retain(|_, memory| {
            within_retention_window(
                memory.heard_tick,
                current_tick,
                profile.conversation_memory_retention_ticks,
            )
        });

        enforce_memory_lane_capacity(
            &mut self.told_beliefs,
            usize::from(profile.conversation_memory_capacity),
            |memory| memory.told_tick,
        );
        enforce_memory_lane_capacity(
            &mut self.heard_beliefs,
            usize::from(profile.conversation_memory_capacity),
            |memory| memory.heard_tick,
        );
    }

    pub fn enforce_ask_witness_memory(&mut self, current_tick: Tick, retention_ticks: u32) {
        self.asked_witnesses.retain(|_, memory| {
            within_retention_window(memory.asked_tick, current_tick, u64::from(retention_ticks))
        });
    }

    #[must_use]
    pub fn told_belief_memory(
        &self,
        key: &TellMemoryKey,
        current_tick: Tick,
        profile: &TellProfile,
    ) -> Option<&ToldBeliefMemory> {
        self.told_beliefs.get(key).filter(|memory| {
            within_retention_window(
                memory.told_tick,
                current_tick,
                profile.conversation_memory_retention_ticks,
            )
        })
    }

    #[must_use]
    fn told_belief_memory_for_topic(
        &self,
        key: &TellMemoryKey,
        current_tick: Tick,
        profile: &TellProfile,
    ) -> Option<&ToldBeliefMemory> {
        self.told_beliefs
            .iter()
            .filter(|(memory_key, _)| {
                memory_key.counterparty == key.counterparty
                    && tell_topic_same_memory_lane(&memory_key.topic, &key.topic)
            })
            .filter_map(|(_, memory)| {
                within_retention_window(
                    memory.told_tick,
                    current_tick,
                    profile.conversation_memory_retention_ticks,
                )
                .then_some(memory)
            })
            .max_by_key(|memory| memory.told_tick)
    }

    #[must_use]
    pub fn heard_belief_memory(
        &self,
        key: &TellMemoryKey,
        current_tick: Tick,
        profile: &TellProfile,
    ) -> Option<&HeardBeliefMemory> {
        self.heard_beliefs.get(key).filter(|memory| {
            within_retention_window(
                memory.heard_tick,
                current_tick,
                profile.conversation_memory_retention_ticks,
            )
        })
    }

    #[must_use]
    pub fn ask_witness_memory(
        &self,
        key: &AskWitnessMemoryKey,
        current_tick: Tick,
        retention_ticks: u32,
    ) -> Option<&AskWitnessMemory> {
        self.asked_witnesses.get(key).filter(|memory| {
            within_retention_window(memory.asked_tick, current_tick, u64::from(retention_ticks))
        })
    }

    #[must_use]
    pub fn recipient_knowledge_status(
        &self,
        key: &TellMemoryKey,
        current_topic_state: &SharedTellState,
        current_tick: Tick,
        profile: &TellProfile,
    ) -> RecipientKnowledgeStatus {
        match self.told_belief_memory_for_topic(key, current_tick, profile) {
            Some(memory) if memory.shared_state == *current_topic_state => {
                RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
            }
            Some(memory) if shared_tell_content_eq(&memory.shared_state, current_topic_state) => {
                RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
            }
            Some(_) => RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief,
            None if self.told_beliefs.keys().any(|memory_key| {
                memory_key.counterparty == key.counterparty
                    && tell_topic_same_memory_lane(&memory_key.topic, &key.topic)
            }) =>
            {
                RecipientKnowledgeStatus::SpeakerPreviouslyToldButMemoryExpired
            }
            None => RecipientKnowledgeStatus::UnknownToSpeaker,
        }
    }

    #[must_use]
    pub fn shared_belief_snapshot_for_subject(
        &self,
        subject: EntityId,
        state: &BelievedEntityState,
        _max_relay_chain_len: u8,
    ) -> SharedTellState {
        let _ = subject;
        SharedTellState::EntityBelief(to_shared_belief_snapshot(state))
    }

    #[must_use]
    pub fn relayable_institutional_beliefs(
        &self,
        max_relay_chain_len: u8,
    ) -> Vec<BelievedInstitutionalClaim> {
        current_institutional_belief_topics(self.institutional_beliefs.values().flat_map(
            |beliefs| {
                beliefs
                    .iter()
                    .filter(|belief| {
                        institutional_knowledge_chain_len(belief.source) <= max_relay_chain_len
                    })
                    .cloned()
            },
        ))
    }

    #[must_use]
    pub fn shared_institutional_belief_for_claim(
        &self,
        claim: InstitutionalClaim,
        max_relay_chain_len: u8,
    ) -> Option<SharedInstitutionalBelief> {
        self.relayable_institutional_beliefs(max_relay_chain_len)
            .into_iter()
            .filter(|belief| institutional_claim_same_memory_lane(belief.claim, claim))
            .max_by_key(institutional_tell_rank)
            .map(|belief| SharedInstitutionalBelief {
                claim: belief.claim,
                source: belief.source,
            })
    }

    #[must_use]
    pub fn shared_tell_state_for_topic(
        &self,
        topic: &TellTopic,
        max_relay_chain_len: u8,
    ) -> Option<SharedTellState> {
        match topic {
            TellTopic::EntityBelief { subject } => self.get_entity(subject).map(|state| {
                self.shared_belief_snapshot_for_subject(*subject, state, max_relay_chain_len)
            }),
            TellTopic::SocialObservation { observation } => {
                (self.social_observations.contains(observation)
                    && social_observation_is_relayable(observation))
                .then_some(SharedTellState::SocialObservation(*observation))
            }
            TellTopic::InstitutionalClaim { claim } => self
                .shared_institutional_belief_for_claim(*claim, max_relay_chain_len)
                .map(SharedTellState::InstitutionalClaim),
        }
    }

    #[must_use]
    pub fn relayable_social_observations(&self, max_relay_chain_len: u8) -> Vec<SocialObservation> {
        self.social_observations
            .iter()
            .copied()
            .filter(|observation| {
                social_observation_is_relayable(observation)
                    && perception_chain_len(observation.source) <= max_relay_chain_len
            })
            .collect()
    }

    #[must_use]
    pub fn believed_office_holder(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        derive_institutional_read(
            self.institutional_beliefs
                .get(&InstitutionalBeliefKey::OfficeHolderOf { office })
                .into_iter()
                .flatten(),
            |claim| match claim {
                InstitutionalClaim::OfficeHolder {
                    office: claim_office,
                    holder,
                    ..
                } if *claim_office == office => Some(*holder),
                _ => None,
            },
        )
    }

    #[must_use]
    pub fn believed_force_controller(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<(Option<EntityId>, bool)> {
        derive_institutional_read(
            self.institutional_beliefs
                .get(&InstitutionalBeliefKey::ForceControllerOf { office })
                .into_iter()
                .flatten(),
            |claim| match claim {
                InstitutionalClaim::ForceControl {
                    office: claim_office,
                    controller,
                    contested,
                    ..
                } if *claim_office == office => Some((*controller, *contested)),
                _ => None,
            },
        )
    }

    #[must_use]
    pub fn believed_membership(
        &self,
        faction: EntityId,
        member: EntityId,
    ) -> InstitutionalBeliefRead<bool> {
        derive_institutional_read(
            self.institutional_beliefs
                .get(&InstitutionalBeliefKey::FactionMembersOf { faction })
                .into_iter()
                .flatten(),
            |claim| match claim {
                InstitutionalClaim::FactionMembership {
                    faction: claim_faction,
                    member: claim_member,
                    active,
                    ..
                } if *claim_faction == faction && *claim_member == member => Some(*active),
                _ => None,
            },
        )
    }

    #[must_use]
    pub fn believed_faction_rally_point(
        &self,
        faction: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        derive_institutional_read(
            self.institutional_beliefs
                .get(&InstitutionalBeliefKey::FactionRallyPointOf { faction })
                .into_iter()
                .flatten(),
            |claim| match claim {
                InstitutionalClaim::FactionRallyPoint {
                    faction: claim_faction,
                    rally_place,
                    ..
                } if *claim_faction == faction => Some(*rally_place),
                _ => None,
            },
        )
    }

    #[must_use]
    pub fn believed_support_declaration(
        &self,
        office: EntityId,
        supporter: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        derive_institutional_read(
            self.institutional_beliefs
                .get(&InstitutionalBeliefKey::SupportFor { supporter, office })
                .into_iter()
                .flatten(),
            |claim| match claim {
                InstitutionalClaim::SupportDeclaration {
                    office: claim_office,
                    supporter: claim_supporter,
                    candidate,
                    ..
                } if *claim_office == office && *claim_supporter == supporter => Some(*candidate),
                _ => None,
            },
        )
    }

    #[must_use]
    pub fn believed_missing_person_status(
        &self,
        subject: EntityId,
    ) -> InstitutionalBeliefRead<MissingPersonReportStatus> {
        derive_institutional_read(
            self.institutional_beliefs
                .get(&InstitutionalBeliefKey::MissingPersonStatus { subject })
                .into_iter()
                .flatten(),
            |claim| match claim {
                InstitutionalClaim::MissingPersonStatus {
                    subject: claim_subject,
                    status,
                    ..
                } if *claim_subject == subject => Some(*status),
                _ => None,
            },
        )
    }

    #[must_use]
    pub fn believed_support_declarations_for_office(
        &self,
        office: EntityId,
    ) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)> {
        self.institutional_beliefs
            .iter()
            .filter_map(|(key, _)| match *key {
                InstitutionalBeliefKey::SupportFor {
                    supporter,
                    office: belief_office,
                } if belief_office == office => Some(supporter),
                _ => None,
            })
            .filter_map(|supporter| {
                let read = self.believed_support_declaration(office, supporter);
                match read {
                    InstitutionalBeliefRead::Unknown => None,
                    _ => Some((supporter, read)),
                }
            })
            .collect()
    }

    fn enforce_institutional_capacity(&mut self, profile: &PerceptionProfile) {
        let capacity = profile.institutional_memory_capacity as usize;
        if capacity == 0 {
            self.institutional_beliefs.clear();
            return;
        }

        while self.total_institutional_beliefs() > capacity {
            let Some((key, index)) = self.oldest_institutional_belief_position() else {
                break;
            };
            let remove_key = {
                let beliefs = self
                    .institutional_beliefs
                    .get_mut(&key)
                    .expect("selected institutional belief key should still exist");
                beliefs.remove(index);
                beliefs.is_empty()
            };
            if remove_key {
                self.institutional_beliefs.remove(&key);
            }
        }
    }

    fn total_institutional_beliefs(&self) -> usize {
        self.institutional_beliefs
            .values()
            .map(std::vec::Vec::len)
            .sum()
    }

    fn oldest_institutional_belief_position(&self) -> Option<(InstitutionalBeliefKey, usize)> {
        self.institutional_beliefs
            .iter()
            .flat_map(|(key, beliefs)| {
                beliefs
                    .iter()
                    .enumerate()
                    .map(move |(index, belief)| (belief.learned_tick, *key, index))
            })
            .min()
            .map(|(_, key, index)| (key, index))
    }

    /// Iterate over all known entity beliefs.
    pub fn iter_known_entities(&self) -> impl Iterator<Item = (&EntityId, &BelievedEntityState)> {
        self.known_entities.iter()
    }

    /// Get the raw entity claims for a subject.
    #[must_use]
    pub fn get_entity_claims(&self, id: &EntityId) -> Option<&[EntityBeliefClaim]> {
        self.entity_claims.get(id).map(Vec::as_slice)
    }

    /// Iterate over all social observations.
    pub fn iter_social_observations(&self) -> impl Iterator<Item = &SocialObservation> {
        self.social_observations.iter()
    }

    /// Get raw institutional beliefs for a key.
    #[must_use]
    pub fn get_institutional_beliefs(
        &self,
        key: &InstitutionalBeliefKey,
    ) -> Option<&[BelievedInstitutionalClaim]> {
        self.institutional_beliefs.get(key).map(Vec::as_slice)
    }

    /// Check whether any institutional belief exists for a key.
    #[must_use]
    pub fn has_institutional_belief(&self, key: &InstitutionalBeliefKey) -> bool {
        self.institutional_beliefs.contains_key(key)
    }

    /// Update the believed activity for a known entity.
    /// Returns `true` if the belief was actually changed.
    /// Returns `false` if the entity is not known or the activity was already equal.
    pub fn update_believed_activity(
        &mut self,
        id: &EntityId,
        activity: Option<BelievedActivity>,
        current_tick: Tick,
        policy: &BeliefConfidencePolicy,
    ) -> bool {
        if self
            .known_entities
            .get(id)
            .is_some_and(|belief| belief.believed_activity != activity)
        {
            self.record_entity_aspect_claim(
                *id,
                EntityBeliefAspect::Activity,
                ClaimValue::Activity(activity),
                PerceptionSource::DirectObservation,
                current_tick,
                Some(current_tick),
                policy,
            );
            return true;
        }
        false
    }

    /// Clear the believed activity for a known entity (set to None).
    /// Returns `true` if it was previously Some.
    pub fn clear_believed_activity(
        &mut self,
        id: &EntityId,
        current_tick: Tick,
        policy: &BeliefConfidencePolicy,
    ) -> bool {
        if self
            .known_entities
            .get(id)
            .is_some_and(|belief| belief.believed_activity.is_some())
        {
            self.record_entity_aspect_claim(
                *id,
                EntityBeliefAspect::Activity,
                ClaimValue::Activity(None),
                PerceptionSource::DirectObservation,
                current_tick,
                Some(current_tick),
                policy,
            );
            return true;
        }
        false
    }

    /// Project a departed subject's believed place to a visible travel destination.
    /// Returns `true` if the known entity existed and the projection was applied.
    pub fn update_departure_projection(
        &mut self,
        id: &EntityId,
        destination: EntityId,
        observed_tick: Tick,
        policy: &BeliefConfidencePolicy,
    ) -> bool {
        if self.known_entities.contains_key(id) {
            self.record_entity_aspect_claim(
                *id,
                EntityBeliefAspect::Location,
                ClaimValue::Place(Some(destination)),
                PerceptionSource::DirectObservation,
                observed_tick,
                Some(observed_tick),
                policy,
            );
            self.reinforce_entity_presentation(
                *id,
                observed_tick,
                BelievedEntityState::MAX_PRESENTATION_TICKS,
            );
            return true;
        }
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn record_entity_aspect_claim(
        &mut self,
        subject: EntityId,
        aspect: EntityBeliefAspect,
        value: ClaimValue,
        source: PerceptionSource,
        current_tick: Tick,
        claimed_event_tick: Option<Tick>,
        policy: &BeliefConfidencePolicy,
    ) {
        self.ensure_claim_backing_from_summary(subject, current_tick, policy);
        self.record_entity_claim(EntityBeliefClaim {
            claim_id: self.next_claim_id,
            subject,
            aspect,
            value,
            source,
            acquired_tick: current_tick,
            claimed_event_tick,
            confidence: belief_confidence(&source, 0, policy),
            refuted_at_tick: None,
        });
        self.refresh_entity_summary_from_claims(subject, current_tick, policy);
    }

    fn ensure_claim_backing_from_summary(
        &mut self,
        subject: EntityId,
        fallback_tick: Tick,
        policy: &BeliefConfidencePolicy,
    ) {
        let Some(summary) = self.known_entities.get(&subject).cloned() else {
            return;
        };
        let existing_aspects = self
            .entity_claims
            .get(&subject)
            .map(|claims| {
                claims
                    .iter()
                    .map(|claim| claim.aspect)
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        let observed_tick = summary.last_observed_tick().unwrap_or(fallback_tick);
        let confidence = belief_confidence(&summary.source, 0, policy);
        for (aspect, value) in entity_claims_for_snapshot(&summary, None) {
            if existing_aspects.contains(&aspect) {
                continue;
            }
            self.record_entity_claim(EntityBeliefClaim {
                claim_id: self.next_claim_id,
                subject,
                aspect,
                value,
                source: summary.source,
                acquired_tick: observed_tick,
                claimed_event_tick: Some(observed_tick),
                confidence,
                refuted_at_tick: None,
            });
        }
    }

    fn reinforce_entity_presentation(
        &mut self,
        subject: EntityId,
        observed_tick: Tick,
        observation_buffer_capacity: u8,
    ) {
        if let Some(summary) = self.known_entities.get_mut(&subject) {
            summary.push_presentation_tick(observed_tick, observation_buffer_capacity);
        }
    }
}

fn entity_claims_for_snapshot(
    snapshot: &BelievedEntityState,
    prior_summary: Option<&BelievedEntityState>,
) -> Vec<(EntityBeliefAspect, ClaimValue)> {
    let mut claims = vec![(
        EntityBeliefAspect::Location,
        ClaimValue::Place(snapshot.last_known_place),
    )];
    if !snapshot.alive || prior_summary.is_none_or(|prior| prior.alive != snapshot.alive) {
        claims.push((EntityBeliefAspect::Alive, ClaimValue::Bool(snapshot.alive)));
    }
    if !snapshot.wounds.is_empty() || prior_summary.is_some_and(|prior| !prior.wounds.is_empty()) {
        claims.push((
            EntityBeliefAspect::Wounded,
            ClaimValue::WoundSnapshot(snapshot.wounds.clone()),
        ));
    }
    if snapshot.believed_activity.is_some()
        || prior_summary.is_some_and(|prior| prior.believed_activity.is_some())
    {
        claims.push((
            EntityBeliefAspect::Activity,
            ClaimValue::Activity(snapshot.believed_activity.clone()),
        ));
    }
    if snapshot.workstation_tag.is_some()
        || prior_summary.is_some_and(|prior| prior.workstation_tag.is_some())
    {
        claims.push((
            EntityBeliefAspect::WorkstationPresent,
            ClaimValue::WorkstationTag(snapshot.workstation_tag),
        ));
    }
    if snapshot.believed_contention.is_some()
        || prior_summary.is_some_and(|prior| prior.believed_contention.is_some())
    {
        claims.push((
            EntityBeliefAspect::ContentionState,
            ClaimValue::ContentionState(snapshot.believed_contention),
        ));
    }
    if snapshot.wash_basin_state.is_some()
        || prior_summary.is_some_and(|prior| prior.wash_basin_state.is_some())
    {
        claims.push((
            EntityBeliefAspect::WashBasinState,
            ClaimValue::WashBasinState(snapshot.wash_basin_state),
        ));
    }
    if snapshot.believed_artifact.is_some()
        || prior_summary.is_some_and(|prior| prior.believed_artifact.is_some())
    {
        claims.push((
            EntityBeliefAspect::ArtifactState,
            ClaimValue::ArtifactState(snapshot.believed_artifact.clone()),
        ));
    }
    if snapshot.last_known_courage.is_some()
        || prior_summary.is_some_and(|prior| prior.last_known_courage.is_some())
    {
        claims.push((
            EntityBeliefAspect::Courage,
            ClaimValue::Courage(snapshot.last_known_courage),
        ));
    }
    if snapshot.believed_evidence.is_some()
        || prior_summary.is_some_and(|prior| prior.believed_evidence.is_some())
    {
        claims.push((
            EntityBeliefAspect::Evidence,
            ClaimValue::EvidenceState(snapshot.believed_evidence.clone()),
        ));
    }

    let mut commodities = snapshot
        .last_known_inventory
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    if let Some(prior) = prior_summary {
        commodities.extend(prior.last_known_inventory.keys().copied());
    }
    for commodity in commodities {
        claims.push((
            EntityBeliefAspect::Inventory(commodity),
            ClaimValue::Quantity(
                snapshot
                    .last_known_inventory
                    .get(&commodity)
                    .copied()
                    .unwrap_or(Quantity(0)),
            ),
        ));
    }

    let prior_resource = prior_summary.and_then(|prior| {
        prior
            .resource_source
            .as_ref()
            .map(|source| source.commodity)
    });
    let current_resource = snapshot
        .resource_source
        .as_ref()
        .map(|source| source.commodity);
    if let Some(prior_commodity) = prior_resource
        && current_resource != Some(prior_commodity)
    {
        claims.push((
            EntityBeliefAspect::ResourceAvailable(prior_commodity),
            ClaimValue::ResourceSource(None),
        ));
    }
    if let Some(current_commodity) = current_resource {
        claims.push((
            EntityBeliefAspect::ResourceAvailable(current_commodity),
            ClaimValue::ResourceSource(snapshot.resource_source.clone()),
        ));
    }

    claims
}

impl Component for AgentBeliefStore {}

/// Compact structural diff between two `AgentBeliefStore` instances.
///
/// Captures only the mutations (added, removed, changed entries) rather than
/// full snapshots. Used by event-log delta compaction to reduce memory.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefStoreDiff {
    pub next_claim_id: Option<ClaimId>,
    pub known_entities_set: Vec<(EntityId, BelievedEntityState)>,
    pub known_entities_removed: Vec<EntityId>,
    pub social_observations_added: Vec<SocialObservation>,
    pub social_observations_removed_count: u16,
    #[serde(default)]
    pub omission_log_added: Vec<ObservationOmission>,
    #[serde(default)]
    pub omission_log_removed_count: u16,
    pub told_beliefs_set: Vec<(TellMemoryKey, ToldBeliefMemory)>,
    pub told_beliefs_removed: Vec<TellMemoryKey>,
    pub heard_beliefs_set: Vec<(TellMemoryKey, HeardBeliefMemory)>,
    pub heard_beliefs_removed: Vec<TellMemoryKey>,
    pub asked_witnesses_set: Vec<(AskWitnessMemoryKey, AskWitnessMemory)>,
    pub asked_witnesses_removed: Vec<AskWitnessMemoryKey>,
    pub place_visits_set: Vec<(EntityId, PlaceVisitRecord)>,
    pub place_visits_removed: Vec<EntityId>,
    pub entity_claims_set: Vec<(EntityId, Vec<EntityBeliefClaim>)>,
    pub entity_claims_removed: Vec<EntityId>,
    pub institutional_beliefs_set: Vec<(InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>)>,
    pub institutional_beliefs_removed: Vec<InstitutionalBeliefKey>,
}

impl BeliefStoreDiff {
    /// Compute the structural diff that transforms `before` into `after`.
    ///
    /// Invariant: `Self::compute(before, after).apply(before) == *after`
    #[must_use]
    pub fn compute(before: &AgentBeliefStore, after: &AgentBeliefStore) -> Self {
        let next_claim_id =
            (before.next_claim_id != after.next_claim_id).then_some(after.next_claim_id);

        let known_entities_set = diff_btree_map_set(&before.known_entities, &after.known_entities);
        let known_entities_removed =
            diff_btree_map_removed(&before.known_entities, &after.known_entities);

        // Social observations are append-heavy with front eviction.
        // We record how many were removed from the front and which were added at the tail.
        let (social_observations_removed_count, social_observations_added) =
            diff_append_evict_slice(&before.social_observations, &after.social_observations);
        let before_omissions: Vec<_> = before
            .observation_omission_log
            .entries
            .iter()
            .copied()
            .collect();
        let after_omissions: Vec<_> = after
            .observation_omission_log
            .entries
            .iter()
            .copied()
            .collect();
        let (omission_log_removed_count, omission_log_added) =
            diff_append_evict_slice(&before_omissions, &after_omissions);

        let told_beliefs_set = diff_btree_map_set(&before.told_beliefs, &after.told_beliefs);
        let told_beliefs_removed =
            diff_btree_map_removed(&before.told_beliefs, &after.told_beliefs);

        let heard_beliefs_set = diff_btree_map_set(&before.heard_beliefs, &after.heard_beliefs);
        let heard_beliefs_removed =
            diff_btree_map_removed(&before.heard_beliefs, &after.heard_beliefs);

        let asked_witnesses_set =
            diff_btree_map_set(&before.asked_witnesses, &after.asked_witnesses);
        let asked_witnesses_removed =
            diff_btree_map_removed(&before.asked_witnesses, &after.asked_witnesses);

        let place_visits_set = diff_btree_map_set(&before.place_visits, &after.place_visits);
        let place_visits_removed =
            diff_btree_map_removed(&before.place_visits, &after.place_visits);

        let entity_claims_set = diff_btree_map_set(&before.entity_claims, &after.entity_claims);
        let entity_claims_removed =
            diff_btree_map_removed(&before.entity_claims, &after.entity_claims);

        let institutional_beliefs_set =
            diff_btree_map_set(&before.institutional_beliefs, &after.institutional_beliefs);
        let institutional_beliefs_removed =
            diff_btree_map_removed(&before.institutional_beliefs, &after.institutional_beliefs);

        Self {
            next_claim_id,
            known_entities_set,
            known_entities_removed,
            social_observations_added,
            social_observations_removed_count,
            omission_log_added,
            omission_log_removed_count,
            told_beliefs_set,
            told_beliefs_removed,
            heard_beliefs_set,
            heard_beliefs_removed,
            asked_witnesses_set,
            asked_witnesses_removed,
            place_visits_set,
            place_visits_removed,
            entity_claims_set,
            entity_claims_removed,
            institutional_beliefs_set,
            institutional_beliefs_removed,
        }
    }

    /// Apply this diff to `base`, producing the `after` state.
    ///
    /// Invariant: `BeliefStoreDiff::compute(before, after).apply(before) == *after`
    #[must_use]
    pub fn apply(self, base: &AgentBeliefStore) -> AgentBeliefStore {
        let mut result = base.clone();

        if let Some(claim_id) = self.next_claim_id {
            result.next_claim_id = claim_id;
        }

        for id in &self.known_entities_removed {
            result.known_entities.remove(id);
        }
        for (id, state) in self.known_entities_set {
            result.known_entities.insert(id, state);
        }

        // Social observations: remove from front, append to tail.
        let remove = self.social_observations_removed_count as usize;
        if remove > 0 {
            result
                .social_observations
                .drain(..remove.min(result.social_observations.len()));
        }
        result
            .social_observations
            .extend(self.social_observations_added);
        let remove = self.omission_log_removed_count as usize;
        if remove > 0 {
            result
                .observation_omission_log
                .entries
                .drain(..remove.min(result.observation_omission_log.entries.len()));
        }
        result
            .observation_omission_log
            .entries
            .extend(self.omission_log_added);

        for key in &self.told_beliefs_removed {
            result.told_beliefs.remove(key);
        }
        for (key, memory) in self.told_beliefs_set {
            result.told_beliefs.insert(key, memory);
        }

        for key in &self.heard_beliefs_removed {
            result.heard_beliefs.remove(key);
        }
        for (key, memory) in self.heard_beliefs_set {
            result.heard_beliefs.insert(key, memory);
        }

        for key in &self.asked_witnesses_removed {
            result.asked_witnesses.remove(key);
        }
        for (key, memory) in self.asked_witnesses_set {
            result.asked_witnesses.insert(key, memory);
        }

        for id in &self.place_visits_removed {
            result.place_visits.remove(id);
        }
        for (id, record) in self.place_visits_set {
            result.place_visits.insert(id, record);
        }

        for id in &self.entity_claims_removed {
            result.entity_claims.remove(id);
        }
        for (id, claims) in self.entity_claims_set {
            result.entity_claims.insert(id, claims);
        }

        for key in &self.institutional_beliefs_removed {
            result.institutional_beliefs.remove(key);
        }
        for (key, claims) in self.institutional_beliefs_set {
            result.institutional_beliefs.insert(key, claims);
        }

        result
    }

    /// Returns `true` if this diff represents no changes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.next_claim_id.is_none()
            && self.known_entities_set.is_empty()
            && self.known_entities_removed.is_empty()
            && self.social_observations_added.is_empty()
            && self.social_observations_removed_count == 0
            && self.omission_log_added.is_empty()
            && self.omission_log_removed_count == 0
            && self.told_beliefs_set.is_empty()
            && self.told_beliefs_removed.is_empty()
            && self.heard_beliefs_set.is_empty()
            && self.heard_beliefs_removed.is_empty()
            && self.asked_witnesses_set.is_empty()
            && self.asked_witnesses_removed.is_empty()
            && self.place_visits_set.is_empty()
            && self.place_visits_removed.is_empty()
            && self.entity_claims_set.is_empty()
            && self.entity_claims_removed.is_empty()
            && self.institutional_beliefs_set.is_empty()
            && self.institutional_beliefs_removed.is_empty()
    }
}

/// Compute entries that are new or changed in `after` relative to `before`.
fn diff_btree_map_set<K: Clone + Ord, V: Clone + PartialEq>(
    before: &BTreeMap<K, V>,
    after: &BTreeMap<K, V>,
) -> Vec<(K, V)> {
    after
        .iter()
        .filter(|(k, v)| before.get(k) != Some(v))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Compute keys that were removed from `before` (present in `before`, absent in `after`).
fn diff_btree_map_removed<K: Clone + Ord, V>(
    before: &BTreeMap<K, V>,
    after: &BTreeMap<K, V>,
) -> Vec<K> {
    before
        .keys()
        .filter(|k| !after.contains_key(k))
        .cloned()
        .collect()
}

/// Diff social observations (`Vec`-based, not `BTreeMap`).
///
/// Social observations follow an append+evict pattern: new observations are
/// pushed to the back, and capacity enforcement removes from the front.
/// We diff by finding the longest common suffix between before and after,
/// then recording how many were removed from the front and which were added
/// at the tail.
fn diff_append_evict_slice<T>(before: &[T], after: &[T]) -> (u16, Vec<T>)
where
    T: Copy + PartialEq,
{
    // Observations are evicted from the front and added to the back.
    // The surviving entries from `before` appear as a prefix of `after`.
    // Find the longest suffix of `before` that matches a prefix of `after`.
    let mut surviving = 0;
    for candidate_removed in 0..=before.len() {
        let remaining = &before[candidate_removed..];
        if after.len() >= remaining.len() && after[..remaining.len()] == *remaining {
            surviving = remaining.len();
            break;
        }
    }

    let removed = u16::try_from(before.len() - surviving).unwrap_or(u16::MAX);
    let added = after[surviving..].to_vec();

    let mut reconstructed = before.to_vec();
    if removed > 0 {
        reconstructed.drain(..usize::from(removed).min(reconstructed.len()));
    }
    reconstructed.extend(added.iter().copied());

    if reconstructed == after {
        (removed, added)
    } else {
        (
            u16::try_from(before.len()).unwrap_or(u16::MAX),
            after.to_vec(),
        )
    }
}

fn derive_institutional_read<'a, T>(
    beliefs: impl IntoIterator<Item = BelievedInstitutionalClaimRef<'a>>,
    extract: impl Fn(&InstitutionalClaim) -> Option<T>,
) -> InstitutionalBeliefRead<T>
where
    T: Copy + Ord,
{
    let values = beliefs
        .into_iter()
        .filter_map(|belief| extract(&belief.claim))
        .collect::<BTreeSet<_>>();

    match values.len() {
        0 => InstitutionalBeliefRead::Unknown,
        1 => InstitutionalBeliefRead::Certain(
            *values
                .iter()
                .next()
                .expect("single-value institutional belief read should contain a value"),
        ),
        _ => InstitutionalBeliefRead::Conflicted(values.into_iter().collect()),
    }
}

type BelievedInstitutionalClaimRef<'a> = &'a BelievedInstitutionalClaim;

#[must_use]
pub fn current_institutional_belief_topics(
    beliefs: impl IntoIterator<Item = BelievedInstitutionalClaim>,
) -> Vec<BelievedInstitutionalClaim> {
    let mut by_topic = BTreeMap::<InstitutionalTellTopicKey, BelievedInstitutionalClaim>::new();

    for belief in beliefs {
        let topic = institutional_tell_topic_key(belief.claim);
        match by_topic.entry(topic) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(belief);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                if institutional_tell_rank(&belief) > institutional_tell_rank(entry.get()) {
                    entry.insert(belief);
                }
            }
        }
    }

    by_topic.into_values().collect()
}

fn institutional_tell_topic_key(claim: InstitutionalClaim) -> InstitutionalTellTopicKey {
    match claim {
        InstitutionalClaim::OfficeHolder { office, .. } => {
            InstitutionalTellTopicKey::OfficeHolder { office }
        }
        InstitutionalClaim::ForceControl { office, .. } => {
            InstitutionalTellTopicKey::ForceControl { office }
        }
        InstitutionalClaim::FactionMembership {
            faction, member, ..
        } => InstitutionalTellTopicKey::FactionMembership { faction, member },
        InstitutionalClaim::FactionRallyPoint { faction, .. } => {
            InstitutionalTellTopicKey::FactionRallyPoint { faction }
        }
        InstitutionalClaim::SupportDeclaration {
            supporter, office, ..
        } => InstitutionalTellTopicKey::SupportDeclaration { supporter, office },
        InstitutionalClaim::Accusation {
            accused,
            violation_id,
            ..
        }
        | InstitutionalClaim::Verdict {
            accused,
            violation_id,
            ..
        } => InstitutionalTellTopicKey::CrimeCase {
            accused,
            violation_id,
        },
        InstitutionalClaim::MissingPersonStatus { subject, .. } => {
            InstitutionalTellTopicKey::MissingPersonStatus { subject }
        }
    }
}

fn institutional_tell_rank(
    belief: &BelievedInstitutionalClaim,
) -> (
    Tick,
    std::cmp::Reverse<u8>,
    Tick,
    Option<EntityId>,
    InstitutionalClaim,
) {
    (
        institutional_claim_effective_tick(belief.claim),
        std::cmp::Reverse(institutional_knowledge_chain_len(belief.source)),
        belief.learned_tick,
        belief.learned_at,
        belief.claim,
    )
}

fn institutional_claim_effective_tick(claim: InstitutionalClaim) -> Tick {
    match claim {
        InstitutionalClaim::OfficeHolder { effective_tick, .. }
        | InstitutionalClaim::ForceControl { effective_tick, .. }
        | InstitutionalClaim::FactionMembership { effective_tick, .. }
        | InstitutionalClaim::FactionRallyPoint { effective_tick, .. }
        | InstitutionalClaim::SupportDeclaration { effective_tick, .. }
        | InstitutionalClaim::Accusation { effective_tick, .. }
        | InstitutionalClaim::Verdict { effective_tick, .. }
        | InstitutionalClaim::MissingPersonStatus { effective_tick, .. } => effective_tick,
    }
}

/// Snapshot of what an agent believes about a specific entity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservedEntitySnapshot {
    #[serde(default)]
    pub believed_kind: Option<EntityKind>,
    pub last_known_place: Option<EntityId>,
    pub last_known_inventory: BTreeMap<CommodityKind, Quantity>,
    pub workstation_tag: Option<WorkstationTag>,
    pub resource_source: Option<ResourceSource>,
    /// Last observed `WashBasinState` for a `WashBasin` facility. FND-14A:
    /// physical state is co-located perception only — agents observe the
    /// basin's current state when at its place and store it here. Off-place
    /// planning consults this remembered snapshot so the agent can route
    /// travel-then-wash plans to remote believed basins; the authoritative
    /// wash precondition re-validates the live state at action time so
    /// stale beliefs trigger replan rather than commit-against-stale-state.
    #[serde(default)]
    pub wash_basin_state: Option<WashBasinState>,
    pub alive: bool,
    pub wounds: Vec<Wound>,
    pub courage: Option<Permille>,
    #[serde(default)]
    pub artifact_state: Option<BelievedArtifactState>,
    #[serde(default)]
    pub contention_state: Option<BelievedContentionState>,
    #[serde(default)]
    pub evidence_state: Option<BelievedEvidenceState>,
}

impl ObservedEntitySnapshot {
    #[must_use]
    pub fn to_believed_entity_state(
        &self,
        observed_tick: Tick,
        source: PerceptionSource,
    ) -> BelievedEntityState {
        BelievedEntityState {
            believed_kind: self.believed_kind,
            last_known_place: self.last_known_place,
            last_known_inventory: self.last_known_inventory.clone(),
            workstation_tag: self.workstation_tag,
            resource_source: self.resource_source.clone(),
            wash_basin_state: self.wash_basin_state,
            alive: self.alive,
            wounds: self.wounds.clone(),
            last_known_courage: self.courage,
            believed_activity: None,
            believed_artifact: self.artifact_state.clone(),
            believed_contention: self.contention_state,
            believed_evidence: self.evidence_state.clone(),
            ..BelievedEntityState::single_observation_defaults(observed_tick, source)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelievedActivity {
    pub action_domain: ActionDomain,
    pub target: Option<EntityId>,
    pub observed_tick: Tick,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelievedContentionState {
    pub grant_holder: Option<EntityId>,
    pub queue_length: u32,
    pub observed_tick: Tick,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelievedBountyTerms {
    pub target: BountyTarget,
    pub reward_commodity: CommodityKind,
    pub reward_quantity: Quantity,
    pub claim_place: EntityId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelievedArtifactState {
    pub kind: ArtifactKind,
    pub state: ArtifactState,
    pub issuer: EntityId,
    pub expires_at: Option<Tick>,
    pub bounty_terms: Option<BelievedBountyTerms>,
    pub notice_topic: Option<NoticeTopic>,
    pub observed_tick: Tick,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelievedEvidenceEntry {
    pub kind: EvidenceKind,
    pub freshness: Tick,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelievedEvidenceState {
    pub entries: Vec<BelievedEvidenceEntry>,
    pub observed_tick: Tick,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelievedEntityState {
    #[serde(default)]
    pub believed_kind: Option<EntityKind>,
    pub last_known_place: Option<EntityId>,
    pub last_known_inventory: BTreeMap<CommodityKind, Quantity>,
    pub workstation_tag: Option<WorkstationTag>,
    pub resource_source: Option<ResourceSource>,
    /// Last observed `WashBasinState` for a `WashBasin` facility. See
    /// `ObservedEntitySnapshot::wash_basin_state` for the FND-14A rationale.
    #[serde(default)]
    pub wash_basin_state: Option<WashBasinState>,
    pub alive: bool,
    pub wounds: Vec<Wound>,
    pub last_known_courage: Option<Permille>,
    pub believed_activity: Option<BelievedActivity>,
    #[serde(default)]
    pub believed_artifact: Option<BelievedArtifactState>,
    #[serde(default)]
    pub believed_contention: Option<BelievedContentionState>,
    #[serde(default)]
    pub believed_evidence: Option<BelievedEvidenceState>,
    pub presentation_ticks: [Tick; 8],
    pub presentation_tick_count: u8,
    pub source: PerceptionSource,
}

impl BelievedEntityState {
    const MAX_PRESENTATION_TICKS: u8 = 8;

    #[must_use]
    pub fn single_observation_defaults(observed_tick: Tick, source: PerceptionSource) -> Self {
        let mut state = Self {
            believed_kind: None,
            last_known_place: None,
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            wash_basin_state: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            presentation_ticks: [Tick(0); Self::MAX_PRESENTATION_TICKS as usize],
            presentation_tick_count: 0,
            source,
        };
        state.push_presentation_tick(observed_tick, Self::MAX_PRESENTATION_TICKS);
        state
    }

    #[must_use]
    pub fn last_observed_tick(&self) -> Option<Tick> {
        if self.presentation_tick_count == 0 {
            None
        } else {
            Some(self.presentation_ticks[usize::from(self.presentation_tick_count - 1)])
        }
    }

    pub fn push_presentation_tick(&mut self, tick: Tick, buffer_capacity: u8) {
        let cap = usize::from(buffer_capacity.min(Self::MAX_PRESENTATION_TICKS));
        if cap == 0 {
            return;
        }
        if usize::from(self.presentation_tick_count) < cap {
            self.presentation_ticks[usize::from(self.presentation_tick_count)] = tick;
            self.presentation_tick_count += 1;
            return;
        }

        self.presentation_ticks.copy_within(1..cap, 0);
        self.presentation_ticks[cap - 1] = tick;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum TellTopic {
    EntityBelief { subject: EntityId },
    SocialObservation { observation: SocialObservation },
    InstitutionalClaim { claim: InstitutionalClaim },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TellMemoryKey {
    pub counterparty: EntityId,
    pub topic: TellTopic,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToldBeliefMemory {
    pub shared_state: SharedTellState,
    pub told_tick: Tick,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HeardBeliefMemory {
    pub heard_state: SharedTellState,
    pub heard_tick: Tick,
    pub disposition: HeardBeliefDisposition,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct AskWitnessMemoryKey {
    pub counterparty: EntityId,
    pub topic_entity: Option<EntityId>,
    pub topic_commodity: Option<CommodityKind>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AskWitnessMemory {
    pub asked_tick: Tick,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SharedTellState {
    EntityBelief(SharedBeliefSnapshot),
    SocialObservation(SocialObservation),
    InstitutionalClaim(SharedInstitutionalBelief),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SharedInstitutionalBelief {
    pub claim: InstitutionalClaim,
    pub source: InstitutionalKnowledgeSource,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HeardBeliefDisposition {
    Accepted,
    Rejected,
    AlreadyHeldEqualOrNewer,
    NotInternalized,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SharedBeliefSnapshot {
    pub last_known_place: Option<EntityId>,
    pub last_known_inventory: BTreeMap<CommodityKind, Quantity>,
    pub workstation_tag: Option<WorkstationTag>,
    pub resource_source: Option<ResourceSource>,
    pub alive: bool,
    pub wounds: Vec<Wound>,
    pub last_known_courage: Option<Permille>,
    pub source: PerceptionSource,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RecipientKnowledgeStatus {
    UnknownToSpeaker,
    SpeakerHasAlreadyToldCurrentBelief,
    SpeakerHasOnlyToldStaleBelief,
    SpeakerPreviouslyToldButMemoryExpired,
}

#[must_use]
pub fn to_shared_belief_snapshot(state: &BelievedEntityState) -> SharedBeliefSnapshot {
    SharedBeliefSnapshot {
        last_known_place: state.last_known_place,
        last_known_inventory: state.last_known_inventory.clone(),
        workstation_tag: state.workstation_tag,
        resource_source: state.resource_source.clone(),
        alive: state.alive,
        wounds: state.wounds.clone(),
        last_known_courage: state.last_known_courage,
        source: state.source,
    }
}

#[must_use]
pub fn share_equivalent(
    current_belief: &BelievedEntityState,
    prior_shared_state: &SharedBeliefSnapshot,
) -> bool {
    shared_belief_content_eq(
        &to_shared_belief_snapshot(current_belief),
        prior_shared_state,
    )
}

#[must_use]
pub fn institutional_claim_subject_entity(claim: InstitutionalClaim) -> EntityId {
    match claim {
        InstitutionalClaim::OfficeHolder { office, .. }
        | InstitutionalClaim::ForceControl { office, .. }
        | InstitutionalClaim::SupportDeclaration { office, .. } => office,
        InstitutionalClaim::FactionMembership { faction, .. }
        | InstitutionalClaim::FactionRallyPoint { faction, .. } => faction,
        InstitutionalClaim::Accusation { accused, .. }
        | InstitutionalClaim::Verdict { accused, .. }
        | InstitutionalClaim::MissingPersonStatus {
            subject: accused, ..
        } => accused,
    }
}

#[must_use]
pub fn institutional_knowledge_chain_len(source: InstitutionalKnowledgeSource) -> u8 {
    match source {
        InstitutionalKnowledgeSource::DirectObservation
        | InstitutionalKnowledgeSource::WitnessedEvent
        | InstitutionalKnowledgeSource::RecordConsultation { .. }
        | InstitutionalKnowledgeSource::SelfDeclaration => 0,
        InstitutionalKnowledgeSource::Report { chain_len, .. } => chain_len,
    }
}

fn perception_chain_len(source: PerceptionSource) -> u8 {
    match source {
        PerceptionSource::DirectObservation | PerceptionSource::Inference => 0,
        PerceptionSource::Report { chain_len, .. } | PerceptionSource::Rumor { chain_len } => {
            chain_len
        }
    }
}

#[must_use]
pub fn social_observation_is_relayable(observation: &SocialObservation) -> bool {
    !matches!(
        observation.detail,
        SocialObservationDetail::WitnessedTelling { .. }
    )
}

#[must_use]
pub fn social_observation_is_redundant_for_listener(
    observation: &SocialObservation,
    listener: EntityId,
) -> bool {
    match observation.detail {
        SocialObservationDetail::WitnessedCooperation { actor, counterpart } => {
            actor == listener || counterpart == listener
        }
        SocialObservationDetail::WitnessedConflict { actor, target }
        | SocialObservationDetail::WitnessedObligation { actor, target } => {
            actor == listener || target == listener
        }
        SocialObservationDetail::WitnessedTelling {
            speaker,
            listener: heard_by,
        } => speaker == listener || heard_by == listener,
        SocialObservationDetail::CoPresence { other } => other == listener,
        SocialObservationDetail::WitnessedAbsence { .. }
        | SocialObservationDetail::SuspectedTheft { .. } => false,
    }
}

#[must_use]
pub fn tell_subject_is_directly_observable_by_listener(
    subject: EntityId,
    subject_kind: Option<crate::EntityKind>,
    subject_place: Option<EntityId>,
    listener: EntityId,
    listener_place: Option<EntityId>,
    listener_observation_fidelity: crate::Permille,
) -> bool {
    subject == listener
        || (subject_place == listener_place
            && listener_observation_fidelity.value() > 0
            && matches!(
                subject_kind,
                Some(
                    crate::EntityKind::Agent
                        | crate::EntityKind::ItemLot
                        | crate::EntityKind::UniqueItem
                        | crate::EntityKind::Container
                )
            ))
}

#[must_use]
pub fn recipient_knowledge_status(
    current_state: &SharedTellState,
    prior_tell: Option<&ToldBeliefMemory>,
) -> RecipientKnowledgeStatus {
    match prior_tell {
        None => RecipientKnowledgeStatus::UnknownToSpeaker,
        Some(memory) if shared_tell_content_eq(&memory.shared_state, current_state) => {
            RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
        }
        Some(_) => RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief,
    }
}

fn shared_tell_content_eq(left: &SharedTellState, right: &SharedTellState) -> bool {
    match (left, right) {
        (SharedTellState::EntityBelief(left), SharedTellState::EntityBelief(right)) => {
            shared_belief_content_eq(left, right)
        }
        (SharedTellState::SocialObservation(left), SharedTellState::SocialObservation(right)) => {
            shared_social_observation_content_eq(left, right)
        }
        (SharedTellState::InstitutionalClaim(left), SharedTellState::InstitutionalClaim(right)) => {
            institutional_claim_same_content(left.claim, right.claim)
        }
        _ => false,
    }
}

fn tell_topic_same_memory_lane(left: &TellTopic, right: &TellTopic) -> bool {
    match (left, right) {
        (
            TellTopic::InstitutionalClaim { claim: left_claim },
            TellTopic::InstitutionalClaim { claim: right_claim },
        ) => institutional_claim_same_memory_lane(*left_claim, *right_claim),
        _ => left == right,
    }
}

pub fn institutional_claim_same_memory_lane(
    left: InstitutionalClaim,
    right: InstitutionalClaim,
) -> bool {
    institutional_tell_topic_key(left) == institutional_tell_topic_key(right)
}

fn institutional_claim_same_content(left: InstitutionalClaim, right: InstitutionalClaim) -> bool {
    match (left, right) {
        (
            InstitutionalClaim::OfficeHolder {
                office: left_office,
                holder: left_holder,
                ..
            },
            InstitutionalClaim::OfficeHolder {
                office: right_office,
                holder: right_holder,
                ..
            },
        ) => left_office == right_office && left_holder == right_holder,
        (
            InstitutionalClaim::ForceControl {
                office: left_office,
                controller: left_controller,
                contested: left_contested,
                ..
            },
            InstitutionalClaim::ForceControl {
                office: right_office,
                controller: right_controller,
                contested: right_contested,
                ..
            },
        ) => {
            left_office == right_office
                && left_controller == right_controller
                && left_contested == right_contested
        }
        (
            InstitutionalClaim::FactionMembership {
                faction: left_faction,
                member: left_member,
                active: left_active,
                ..
            },
            InstitutionalClaim::FactionMembership {
                faction: right_faction,
                member: right_member,
                active: right_active,
                ..
            },
        ) => {
            left_faction == right_faction
                && left_member == right_member
                && left_active == right_active
        }
        (
            InstitutionalClaim::FactionRallyPoint {
                faction: left_faction,
                rally_place: left_rally_place,
                ..
            },
            InstitutionalClaim::FactionRallyPoint {
                faction: right_faction,
                rally_place: right_rally_place,
                ..
            },
        ) => left_faction == right_faction && left_rally_place == right_rally_place,
        (
            InstitutionalClaim::SupportDeclaration {
                supporter: left_supporter,
                office: left_office,
                candidate: left_candidate,
                ..
            },
            InstitutionalClaim::SupportDeclaration {
                supporter: right_supporter,
                office: right_office,
                candidate: right_candidate,
                ..
            },
        ) => {
            left_supporter == right_supporter
                && left_office == right_office
                && left_candidate == right_candidate
        }
        (
            InstitutionalClaim::Accusation {
                accuser: left_filer,
                accused: left_target,
                violation_id: left_case_violation_id,
                ..
            },
            InstitutionalClaim::Accusation {
                accuser: right_filer,
                accused: right_target,
                violation_id: right_case_violation_id,
                ..
            },
        ) => {
            left_filer == right_filer
                && left_target == right_target
                && left_case_violation_id == right_case_violation_id
        }
        (
            InstitutionalClaim::Verdict {
                accused: left_accused,
                violation_id: left_violation_id,
                punishment: left_punishment,
                ..
            },
            InstitutionalClaim::Verdict {
                accused: right_accused,
                violation_id: right_violation_id,
                punishment: right_punishment,
                ..
            },
        ) => {
            left_accused == right_accused
                && left_violation_id == right_violation_id
                && left_punishment == right_punishment
        }
        (
            InstitutionalClaim::MissingPersonStatus {
                subject: left_subject,
                status: left_status,
                ..
            },
            InstitutionalClaim::MissingPersonStatus {
                subject: right_subject,
                status: right_status,
                ..
            },
        ) => left_subject == right_subject && left_status == right_status,
        _ => false,
    }
}

fn shared_belief_content_eq(left: &SharedBeliefSnapshot, right: &SharedBeliefSnapshot) -> bool {
    left.last_known_place == right.last_known_place
        && left.last_known_inventory == right.last_known_inventory
        && left.workstation_tag == right.workstation_tag
        && left.resource_source == right.resource_source
        && left.alive == right.alive
        && left.wounds == right.wounds
        && left.last_known_courage == right.last_known_courage
}

fn shared_social_observation_content_eq(
    left: &SocialObservation,
    right: &SocialObservation,
) -> bool {
    left.detail == right.detail
        && left.place == right.place
        && left.observed_tick == right.observed_tick
}

#[must_use]
pub fn build_observed_entity_snapshot(
    world: &World,
    entity: EntityId,
) -> Option<ObservedEntitySnapshot> {
    let believed_kind = Some(world.entity_kind(entity)?);

    let mut inventory = BTreeMap::new();
    for commodity in CommodityKind::ALL {
        let quantity = if let Some(lot) = world.get_component_item_lot(entity) {
            if lot.commodity == commodity {
                lot.quantity
            } else {
                Quantity(0)
            }
        } else {
            world.controlled_commodity_quantity(entity, commodity)
        };
        if quantity > Quantity(0) {
            inventory.insert(commodity, quantity);
        }
    }

    Some(ObservedEntitySnapshot {
        believed_kind,
        last_known_place: world.effective_place(entity),
        last_known_inventory: inventory,
        workstation_tag: world
            .get_component_workstation_marker(entity)
            .map(|marker| marker.0),
        resource_source: world.get_component_resource_source(entity).cloned(),
        wash_basin_state: world.get_component_wash_basin_state(entity).copied(),
        alive: world.get_component_dead_at(entity).is_none(),
        wounds: world
            .get_component_wound_list(entity)
            .map(|wounds| wounds.wounds.clone())
            .unwrap_or_default(),
        courage: world
            .get_component_utility_profile(entity)
            .map(|p| p.courage),
        artifact_state: build_believed_artifact_state(world, entity, Tick(0)),
        contention_state: build_believed_contention_state(world, entity, Tick(0)),
        evidence_state: build_believed_evidence_state(world, entity, Tick(0)),
    })
}

#[must_use]
pub fn build_believed_artifact_state(
    world: &World,
    entity: EntityId,
    observed_tick: Tick,
) -> Option<BelievedArtifactState> {
    let header = world.get_component_artifact_header(entity)?;
    let bounty_terms = world
        .get_component_bounty_terms(entity)
        .map(|terms| BelievedBountyTerms {
            target: terms.target,
            reward_commodity: terms.reward_commodity,
            reward_quantity: terms.reward_quantity,
            claim_place: terms.claim_place,
        });
    let notice_topic = world
        .get_component_notice_content(entity)
        .map(|notice| notice.topic);

    Some(BelievedArtifactState {
        kind: header.kind,
        state: header.state,
        issuer: header.issuer,
        expires_at: header.expires_at,
        bounty_terms,
        notice_topic,
        observed_tick,
    })
}

#[must_use]
pub fn build_believed_contention_state(
    world: &World,
    entity: EntityId,
    observed_tick: Tick,
) -> Option<BelievedContentionState> {
    let queue = world.get_component_contention_queue(entity)?;
    Some(BelievedContentionState {
        grant_holder: queue.granted.as_ref().map(|grant| grant.actor),
        queue_length: u32::try_from(queue.waiting.len())
            .expect("contention queue length should fit in u32"),
        observed_tick,
    })
}

#[must_use]
pub fn build_believed_evidence_state(
    world: &World,
    entity: EntityId,
    observed_tick: Tick,
) -> Option<BelievedEvidenceState> {
    let scene = world.get_component_scene_evidence(entity)?;
    Some(BelievedEvidenceState {
        entries: scene
            .evidence
            .iter()
            .map(|entry| BelievedEvidenceEntry {
                kind: entry.kind,
                freshness: entry.created_at,
            })
            .collect(),
        observed_tick,
    })
}

#[must_use]
pub fn build_believed_entity_state(
    world: &World,
    entity: EntityId,
    observed_tick: Tick,
    source: PerceptionSource,
) -> Option<BelievedEntityState> {
    build_observed_entity_snapshot(world, entity).map(|mut snapshot| {
        if let Some(artifact) = snapshot.artifact_state.as_mut() {
            artifact.observed_tick = observed_tick;
        }
        if let Some(contention) = snapshot.contention_state.as_mut() {
            contention.observed_tick = observed_tick;
        }
        if let Some(evidence) = snapshot.evidence_state.as_mut() {
            evidence.observed_tick = observed_tick;
        }
        snapshot.to_believed_entity_state(observed_tick, source)
    })
}

#[must_use]
pub fn derive_entity_summary(
    claims: &[EntityBeliefClaim],
    current_tick: Tick,
    policy: &BeliefConfidencePolicy,
) -> Option<BelievedEntityState> {
    let mut winners = BTreeMap::<EntityBeliefAspect, &EntityBeliefClaim>::new();

    for claim in claims.iter().filter(|claim| !claim_is_refuted(claim)) {
        match winners.entry(claim.aspect) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(claim);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                if claim_rank(claim, current_tick, policy)
                    > claim_rank(entry.get(), current_tick, policy)
                {
                    entry.insert(claim);
                }
            }
        }
    }

    let metadata_claim = winners
        .values()
        .copied()
        .max_by_key(|claim| claim_rank(claim, current_tick, policy))?;

    let mut summary = BelievedEntityState {
        believed_kind: None,
        last_known_place: None,
        last_known_inventory: BTreeMap::new(),
        workstation_tag: None,
        resource_source: None,
        alive: true,
        wounds: Vec::new(),
        last_known_courage: None,
        believed_activity: None,
        believed_artifact: None,
        believed_contention: None,
        believed_evidence: None,
        ..BelievedEntityState::single_observation_defaults(
            metadata_claim
                .claimed_event_tick
                .unwrap_or(metadata_claim.acquired_tick),
            metadata_claim.source,
        )
    };

    for (aspect, claim) in winners {
        match (aspect, &claim.value) {
            (EntityBeliefAspect::Location, ClaimValue::Place(place)) => {
                summary.last_known_place = *place;
            }
            (EntityBeliefAspect::Inventory(commodity), ClaimValue::Quantity(quantity)) => {
                if *quantity > Quantity(0) {
                    summary.last_known_inventory.insert(commodity, *quantity);
                } else {
                    summary.last_known_inventory.remove(&commodity);
                }
            }
            (EntityBeliefAspect::Alive, ClaimValue::Bool(alive)) => {
                summary.alive = *alive;
            }
            (EntityBeliefAspect::Wounded, ClaimValue::WoundSnapshot(wounds)) => {
                summary.wounds.clone_from(wounds);
            }
            (EntityBeliefAspect::Activity, ClaimValue::Activity(activity)) => {
                summary.believed_activity.clone_from(activity);
            }
            (EntityBeliefAspect::WorkstationPresent, ClaimValue::WorkstationTag(tag)) => {
                summary.workstation_tag = *tag;
            }
            (EntityBeliefAspect::ResourceAvailable(_), ClaimValue::ResourceSource(source)) => {
                summary.resource_source.clone_from(source);
            }
            (EntityBeliefAspect::ContentionState, ClaimValue::ContentionState(contention)) => {
                summary.believed_contention = *contention;
            }
            (EntityBeliefAspect::WashBasinState, ClaimValue::WashBasinState(state)) => {
                summary.wash_basin_state = *state;
            }
            (EntityBeliefAspect::ArtifactState, ClaimValue::ArtifactState(artifact)) => {
                summary.believed_artifact.clone_from(artifact);
            }
            (EntityBeliefAspect::Courage, ClaimValue::Courage(courage)) => {
                summary.last_known_courage = *courage;
            }
            (EntityBeliefAspect::Evidence, ClaimValue::EvidenceState(evidence)) => {
                summary.believed_evidence.clone_from(evidence);
            }
            _ => {}
        }
    }

    Some(summary)
}

fn claim_is_refuted(claim: &EntityBeliefClaim) -> bool {
    claim.refuted_at_tick.is_some()
}

fn preserve_believed_kind(prior: Option<&BelievedEntityState>, summary: &mut BelievedEntityState) {
    if summary.believed_kind.is_none() {
        summary.believed_kind = prior.and_then(|state| state.believed_kind);
    }
}

fn preserve_presentation_history(
    prior: Option<&BelievedEntityState>,
    summary: &mut BelievedEntityState,
) {
    let Some(prior) = prior else {
        return;
    };
    summary.presentation_ticks = prior.presentation_ticks;
    summary.presentation_tick_count = prior.presentation_tick_count;
}

fn claim_rank(
    claim: &EntityBeliefClaim,
    current_tick: Tick,
    policy: &BeliefConfidencePolicy,
) -> (u16, Tick, ClaimId) {
    (
        effective_claim_confidence(claim, current_tick, policy),
        claim.acquired_tick,
        claim.claim_id,
    )
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum SourceKind {
    Direct,
    Report(EntityId),
    Rumor,
    Inference,
}

fn source_kind(source: &PerceptionSource) -> SourceKind {
    match source {
        PerceptionSource::DirectObservation => SourceKind::Direct,
        PerceptionSource::Report { from, .. } => SourceKind::Report(*from),
        PerceptionSource::Rumor { .. } => SourceKind::Rumor,
        PerceptionSource::Inference => SourceKind::Inference,
    }
}

fn claim_staleness_anchor(claim: &EntityBeliefClaim) -> Tick {
    claim.claimed_event_tick.unwrap_or(claim.acquired_tick)
}

fn claim_dominates(left: &EntityBeliefClaim, right: &EntityBeliefClaim) -> bool {
    let left_anchor = claim_staleness_anchor(left);
    let right_anchor = claim_staleness_anchor(right);

    if left.confidence < right.confidence
        || left_anchor < right_anchor
        || left.acquired_tick < right.acquired_tick
        || left.claim_id < right.claim_id
    {
        return false;
    }

    left.confidence > right.confidence
        || left_anchor > right_anchor
        || left.acquired_tick > right.acquired_tick
        || left.claim_id > right.claim_id
}

fn refute_conflicting_claims_for_direct_observation(
    claims: &mut [EntityBeliefClaim],
    refuting_claim: &EntityBeliefClaim,
) {
    let refuted_at_tick = refuting_claim
        .claimed_event_tick
        .unwrap_or(refuting_claim.acquired_tick);
    for existing in claims.iter_mut() {
        if existing.subject != refuting_claim.subject
            || existing.aspect != refuting_claim.aspect
            || existing.value == refuting_claim.value
            || claim_is_refuted(existing)
            || !claim_dominates(refuting_claim, existing)
        {
            continue;
        }
        existing.refuted_at_tick = Some(refuted_at_tick);
    }
}

pub fn effective_claim_confidence(
    claim: &EntityBeliefClaim,
    current_tick: Tick,
    policy: &BeliefConfidencePolicy,
) -> u16 {
    let staleness_anchor = claim.claimed_event_tick.unwrap_or(claim.acquired_tick);
    let staleness_ticks = current_tick.0.saturating_sub(staleness_anchor.0);
    let staleness_penalty = u16::try_from(staleness_ticks)
        .unwrap_or(u16::MAX)
        .saturating_mul(policy.staleness_penalty_per_tick.value());
    claim.confidence.value().saturating_sub(staleness_penalty)
}

/// How the agent acquired a belief snapshot.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum PerceptionSource {
    DirectObservation,
    Report { from: EntityId, chain_len: u8 },
    Rumor { chain_len: u8 },
    Inference,
}

/// Explicit per-agent policy for deriving belief confidence from provenance and age.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefConfidencePolicy {
    pub direct_observation_base: Permille,
    pub report_base: Permille,
    pub rumor_base: Permille,
    pub inference_base: Permille,
    pub report_chain_penalty: Permille,
    pub rumor_chain_penalty: Permille,
    pub staleness_penalty_per_tick: Permille,
}

impl Default for BeliefConfidencePolicy {
    fn default() -> Self {
        Self {
            direct_observation_base: Permille::new(950).unwrap(),
            report_base: Permille::new(780).unwrap(),
            rumor_base: Permille::new(560).unwrap(),
            inference_base: Permille::new(420).unwrap(),
            report_chain_penalty: Permille::new(90).unwrap(),
            rumor_chain_penalty: Permille::new(110).unwrap(),
            staleness_penalty_per_tick: Permille::new(12).unwrap(),
        }
    }
}

/// Derives confidence from provenance and age without storing abstract authority state.
#[must_use]
pub fn belief_confidence(
    source: &PerceptionSource,
    staleness_ticks: u64,
    policy: &BeliefConfidencePolicy,
) -> Permille {
    let base = match *source {
        PerceptionSource::DirectObservation => policy.direct_observation_base.value(),
        PerceptionSource::Report { chain_len, .. } => policy.report_base.value().saturating_sub(
            policy
                .report_chain_penalty
                .value()
                .saturating_mul(u16::from(chain_len.saturating_sub(1))),
        ),
        PerceptionSource::Rumor { chain_len } => policy.rumor_base.value().saturating_sub(
            policy
                .rumor_chain_penalty
                .value()
                .saturating_mul(u16::from(chain_len.saturating_sub(1))),
        ),
        PerceptionSource::Inference => policy.inference_base.value(),
    };
    let staleness_penalty = u16::try_from(staleness_ticks)
        .unwrap_or(u16::MAX)
        .saturating_mul(policy.staleness_penalty_per_tick.value());

    Permille::new(base.saturating_sub(staleness_penalty))
        .expect("belief confidence derivation always yields a valid permille")
}

/// A witnessed social fact retained in belief memory.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SocialObservation {
    pub detail: SocialObservationDetail,
    pub place: EntityId,
    pub observed_tick: Tick,
    pub source: PerceptionSource,
}

impl SocialObservation {
    #[must_use]
    pub const fn kind(&self) -> SocialObservationKind {
        self.detail.kind()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SocialObservationDetail {
    WitnessedCooperation {
        actor: EntityId,
        counterpart: EntityId,
    },
    WitnessedConflict {
        actor: EntityId,
        target: EntityId,
    },
    WitnessedObligation {
        actor: EntityId,
        target: EntityId,
    },
    WitnessedTelling {
        speaker: EntityId,
        listener: EntityId,
    },
    CoPresence {
        other: EntityId,
    },
    WitnessedAbsence {
        missing_entity: EntityId,
        expected_place: EntityId,
    },
    SuspectedTheft {
        theft: TheftFacts,
        suspect: Option<EntityId>,
    },
}

impl SocialObservationDetail {
    #[must_use]
    pub const fn kind(&self) -> SocialObservationKind {
        match self {
            Self::WitnessedCooperation { .. } => SocialObservationKind::WitnessedCooperation,
            Self::WitnessedConflict { .. } => SocialObservationKind::WitnessedConflict,
            Self::WitnessedObligation { .. } => SocialObservationKind::WitnessedObligation,
            Self::WitnessedTelling { .. } => SocialObservationKind::WitnessedTelling,
            Self::CoPresence { .. } => SocialObservationKind::CoPresence,
            Self::WitnessedAbsence { .. } => SocialObservationKind::WitnessedAbsence,
            Self::SuspectedTheft { .. } => SocialObservationKind::SuspectedTheft,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SocialObservationKind {
    WitnessedCooperation,
    WitnessedConflict,
    WitnessedObligation,
    WitnessedTelling,
    CoPresence,
    /// Agent confirmed the absence of an expected entity at a location through investigation.
    WitnessedAbsence,
    /// Agent confirmed an owned entity is missing under theft suspicion.
    SuspectedTheft,
}

/// Concrete differences between a prior belief and a new observation.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum MismatchKind {
    EntityMissing,
    AliveStatusChanged,
    InventoryDiscrepancy {
        commodity: CommodityKind,
        believed: Quantity,
        observed: Quantity,
    },
    ResourceSourceDiscrepancy {
        commodity: CommodityKind,
        believed: Quantity,
        observed: Quantity,
    },
    PlaceChanged {
        believed_place: EntityId,
        observed_place: EntityId,
    },
}

/// Per-agent parameters controlling belief retention and observation quality.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerceptionProfile {
    /// Base accuracy of direct observations. Higher values mean more reliable perception.
    pub observation_fidelity: Permille,
    /// Policy governing how belief confidence is assigned and decays over time.
    pub confidence_policy: BeliefConfidencePolicy,
    /// Maximum number of institutional belief entries the agent retains.
    pub institutional_memory_capacity: u32,
    /// Speed modifier for consulting institutional records (e.g., ledgers, boards).
    pub consultation_speed_factor: Permille,
    /// Threshold below which contradicting evidence causes belief revision.
    pub contradiction_tolerance: Permille,
    /// Age threshold (in ticks) at which stale entity beliefs are pruned by activation decay.
    pub entity_activation_threshold: Permille,
    /// Minimum confidence required for an entity belief claim to be retained.
    pub claim_confidence_threshold: Permille,
    /// Maximum number of pending observations buffered before processing.
    pub observation_buffer_capacity: u8,
    /// Maximum number of co-located entities observed per tick before salience filtering truncates.
    #[serde(default = "default_observation_budget")]
    pub observation_budget: u8,
    /// Policy used to rank observation salience before budget truncation.
    #[serde(default)]
    pub salience_policy: SaliencePolicy,
    /// Maximum number of recent omitted observations retained in the agent's belief store.
    #[serde(default = "default_omission_log_capacity")]
    pub omission_log_capacity: u8,
    /// Observation priority boost for entities relevant to the agent's current needs.
    pub need_salience_boost: Permille,
    /// Need pressure level above which the salience boost activates.
    pub need_salience_urgency_threshold: Permille,
}

impl Component for PerceptionProfile {}

fn default_observation_budget() -> u8 {
    24
}

pub fn default_omission_log_capacity() -> u8 {
    16
}

impl Default for PerceptionProfile {
    fn default() -> Self {
        Self {
            observation_fidelity: Permille::new(875).unwrap(),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: Permille::new(500).unwrap(),
            contradiction_tolerance: Permille::new(300).unwrap(),
            entity_activation_threshold: Permille::new(100).unwrap(),
            claim_confidence_threshold: Permille::new(50).unwrap(),
            observation_buffer_capacity: 5,
            observation_budget: default_observation_budget(),
            salience_policy: SaliencePolicy::default(),
            omission_log_capacity: default_omission_log_capacity(),
            need_salience_boost: Permille::new(500).unwrap(),
            need_salience_urgency_threshold: Permille::new(500).unwrap(),
        }
    }
}

/// Per-agent parameters controlling what information an agent relays and accepts.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TellProfile {
    /// Maximum number of co-located agents this agent considers as tell recipients per tick.
    pub max_tell_candidates: u8,
    /// Maximum relay chain length before the agent stops forwarding heard information.
    pub max_relay_chain_len: u8,
    /// Maximum number of recent conversation entries the agent retains.
    pub conversation_memory_capacity: u16,
    /// How long (in ticks) conversation memory entries are retained before expiry.
    pub conversation_memory_retention_ticks: u64,
}

impl Component for TellProfile {}

impl Default for TellProfile {
    fn default() -> Self {
        Self {
            max_tell_candidates: 3,
            max_relay_chain_len: 3,
            conversation_memory_capacity: 12,
            conversation_memory_retention_ticks: 48,
        }
    }
}

fn enforce_memory_lane_capacity<T, F>(
    lane: &mut BTreeMap<TellMemoryKey, T>,
    capacity: usize,
    tick_of: F,
) where
    F: Fn(&T) -> Tick,
{
    if capacity == 0 {
        lane.clear();
        return;
    }

    let excess = lane.len().saturating_sub(capacity);
    if excess == 0 {
        return;
    }

    let mut eviction_order = lane
        .iter()
        .map(|(key, memory)| (tick_of(memory), *key))
        .collect::<Vec<_>>();
    eviction_order.sort_unstable();

    for (_, key) in eviction_order.into_iter().take(excess) {
        lane.remove(&key);
    }
}

fn within_retention_window(observed_tick: Tick, current_tick: Tick, retention_ticks: u64) -> bool {
    current_tick.0.saturating_sub(observed_tick.0) <= retention_ticks
}

#[must_use]
pub fn compute_activation(current_tick: Tick, presentation_ticks: &[Tick], count: u8) -> u16 {
    let mut total: u32 = 0;
    let count = usize::from(count).min(presentation_ticks.len());
    for tick in presentation_ticks.iter().take(count) {
        let age = current_tick.0.saturating_sub(tick.0).max(1);
        let scaled_root = age.saturating_mul(1_000_000).isqrt().max(1);
        let contribution = 1_000_000u32 / u32::try_from(scaled_root).unwrap();
        total = total.saturating_add(contribution);
    }

    total.min(u32::from(u16::MAX)) as u16
}

#[must_use]
pub fn salience_boost(
    max_need: u16,
    believed_kind: Option<EntityKind>,
    urgency_threshold: Permille,
    boost: Permille,
) -> u16 {
    if believed_kind != Some(EntityKind::ItemLot) {
        return 0;
    }
    if max_need < urgency_threshold.value() {
        return 0;
    }

    (u32::from(max_need) * u32::from(boost.value()) / 1000) as u16
}

#[must_use]
fn carries_pressuring_opportunity(
    needs: &HomeostaticNeeds,
    state: &BelievedEntityState,
    urgency_threshold: Permille,
) -> bool {
    HomeostaticNeedId::ALL
        .iter()
        .copied()
        .filter(|need| needs.value(*need).value() >= urgency_threshold.value())
        .any(|need| carries_opportunity_for_need(need, state))
}

#[must_use]
fn carries_opportunity_for_need(need: HomeostaticNeedId, state: &BelievedEntityState) -> bool {
    match need {
        HomeostaticNeedId::Hunger | HomeostaticNeedId::Thirst => {
            state.workstation_tag.is_some() && resource_source_relieves_need(need, state)
        }
        HomeostaticNeedId::Dirtiness => {
            state.wash_basin_state.is_some()
                && state.workstation_tag == Some(WorkstationTag::WashBasin)
        }
        HomeostaticNeedId::Fatigue | HomeostaticNeedId::Bladder => false,
    }
}

#[must_use]
fn resource_source_relieves_need(need: HomeostaticNeedId, state: &BelievedEntityState) -> bool {
    let Some(source) = state.resource_source.as_ref() else {
        return false;
    };
    let Some(profile) = source.commodity.spec().consumable_profile else {
        return false;
    };

    match need {
        HomeostaticNeedId::Hunger => profile.hunger_relief_per_unit.value() > 0,
        HomeostaticNeedId::Thirst => profile.thirst_relief_per_unit.value() > 0,
        HomeostaticNeedId::Fatigue | HomeostaticNeedId::Bladder | HomeostaticNeedId::Dirtiness => {
            false
        }
    }
}

#[must_use]
fn state_salience_boost(
    needs: &HomeostaticNeeds,
    state: &BelievedEntityState,
    urgency_threshold: Permille,
    boost: Permille,
) -> u16 {
    if state.source == PerceptionSource::DirectObservation
        && carries_pressuring_opportunity(needs, state, urgency_threshold)
    {
        return boost.value();
    }

    salience_boost(
        needs.max_value(),
        state.believed_kind,
        urgency_threshold,
        boost,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        AgentBeliefStore, AskWitnessMemory, AskWitnessMemoryKey, BeliefConfidencePolicy,
        BelievedActivity, BelievedContentionState, BelievedEntityState, BelievedEvidenceEntry,
        BelievedEvidenceState, HeardBeliefDisposition, HeardBeliefMemory, MismatchKind,
        ObservationOmission, ObservationOmissionLog, ObservedEntitySnapshot, OmissionReason,
        PerceptionProfile, PerceptionSource, PlaceVisitRecord, RecipientKnowledgeStatus,
        SaliencePolicy, SharedInstitutionalBelief, SharedTellState, SocialObservation,
        SocialObservationDetail, SocialObservationKind, TellMemoryKey, TellProfile, TellTopic,
        ToldBeliefMemory, belief_confidence, build_believed_entity_state,
        build_observed_entity_snapshot, compute_activation, default_omission_log_capacity,
        derive_entity_summary, recipient_knowledge_status, salience_boost, share_equivalent,
        state_salience_boost, to_shared_belief_snapshot,
    };
    use crate::{
        ActionDefId, ActionDomain, BelievedArtifactState, BelievedBountyTerms,
        BelievedInstitutionalClaim, BodyPart, ClaimId, ClaimValue, CommodityKind, ControlSource,
        DeadAt, DisturbanceKind, EntityBeliefAspect, EntityBeliefClaim, EntityId, EntityKind,
        EvidenceKind, HomeostaticNeedId, HomeostaticNeeds, InstitutionalBeliefKey,
        InstitutionalBeliefRead, InstitutionalClaim, InstitutionalKnowledgeSource, NoticeTopic,
        Permille, Quantity, ResourceSource, SceneEvidence, TheftFacts, Tick, WashBasinState,
        WorkstationTag, World, Wound, WoundCause, WoundId, WoundList, build_prototype_world,
        current_institutional_belief_topics, institutional_claim_same_memory_lane,
        traits::Component,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::collections::BTreeMap;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn profile(
        entity_activation_threshold: u16,
        claim_confidence_threshold: u16,
        observation_buffer_capacity: u8,
    ) -> PerceptionProfile {
        PerceptionProfile {
            observation_fidelity: Permille::new(750).unwrap(),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 9,
            consultation_speed_factor: Permille::new(650).unwrap(),
            contradiction_tolerance: Permille::new(275).unwrap(),
            entity_activation_threshold: Permille::new(entity_activation_threshold).unwrap(),
            claim_confidence_threshold: Permille::new(claim_confidence_threshold).unwrap(),
            observation_buffer_capacity,
            observation_budget: 24,
            salience_policy: SaliencePolicy::default(),
            omission_log_capacity: default_omission_log_capacity(),
            need_salience_boost: Permille::new(500).unwrap(),
            need_salience_urgency_threshold: Permille::new(500).unwrap(),
        }
    }

    fn sample_institutional_belief(observed_tick: u64) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::OfficeHolder {
                office: entity(50),
                holder: Some(entity(51)),
                effective_tick: Tick(observed_tick.saturating_sub(1)),
            },
            source: InstitutionalKnowledgeSource::RecordConsultation {
                record: entity(52),
                entry_id: crate::RecordEntryId(3),
            },
            learned_tick: Tick(observed_tick),
            learned_at: Some(entity(53)),
        }
    }

    fn policy() -> BeliefConfidencePolicy {
        BeliefConfidencePolicy::default()
    }

    fn sample_wound(id: u64, observed_tick: u64) -> Wound {
        Wound {
            id: WoundId(id),
            body_part: BodyPart::Torso,
            cause: WoundCause::Combat {
                attacker: entity(99),
                weapon: crate::CombatWeaponRef::Unarmed,
            },
            severity: Permille::new(125).unwrap(),
            inflicted_at: Tick(observed_tick),
            bleed_rate_per_tick: Permille::new(10).unwrap(),
        }
    }

    fn sample_state(observed_tick: u64, commodity_qty: u32) -> BelievedEntityState {
        let mut inventory = BTreeMap::new();
        inventory.insert(CommodityKind::Apple, Quantity(commodity_qty));
        BelievedEntityState {
            believed_kind: None,
            last_known_place: Some(entity(10)),
            last_known_inventory: inventory,
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: vec![sample_wound(1, observed_tick)],
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                Tick(observed_tick),
                PerceptionSource::DirectObservation,
            )
        }
    }

    fn sample_claim(
        claim_id: u64,
        subject: u32,
        aspect: EntityBeliefAspect,
        value: ClaimValue,
        source: PerceptionSource,
        acquired_tick: u64,
        confidence: u16,
    ) -> EntityBeliefClaim {
        EntityBeliefClaim {
            claim_id: ClaimId(claim_id),
            subject: entity(subject),
            aspect,
            value,
            source,
            acquired_tick: Tick(acquired_tick),
            claimed_event_tick: None,
            confidence: Permille::new(confidence).unwrap(),
            refuted_at_tick: None,
        }
    }

    fn needs_with_pressure(need: HomeostaticNeedId) -> HomeostaticNeeds {
        let mut needs = HomeostaticNeeds::new_sated();
        match need {
            HomeostaticNeedId::Hunger => needs.hunger = Permille::new(1000).unwrap(),
            HomeostaticNeedId::Thirst => needs.thirst = Permille::new(1000).unwrap(),
            HomeostaticNeedId::Fatigue => needs.fatigue = Permille::new(1000).unwrap(),
            HomeostaticNeedId::Bladder => needs.bladder = Permille::new(1000).unwrap(),
            HomeostaticNeedId::Dirtiness => needs.dirtiness = Permille::new(1000).unwrap(),
        }
        needs
    }

    fn resource_source(commodity: CommodityKind) -> ResourceSource {
        ResourceSource {
            commodity,
            available_quantity: Quantity(4),
            max_quantity: Quantity(8),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
        }
    }

    fn claim_backed_store(entity_id: u32, claims: Vec<EntityBeliefClaim>) -> AgentBeliefStore {
        let mut store = AgentBeliefStore::new();
        let subject = entity(entity_id);
        store.entity_claims.insert(subject, claims);
        store.known_entities.insert(
            subject,
            derive_entity_summary(
                store.entity_claims.get(&subject).unwrap(),
                Tick(10),
                &policy(),
            )
            .unwrap(),
        );
        store
    }

    fn sample_social_observation(observed_tick: u64) -> SocialObservation {
        SocialObservation {
            detail: SocialObservationDetail::WitnessedConflict {
                actor: entity(1),
                target: entity(2),
            },
            place: entity(10),
            observed_tick: Tick(observed_tick),
            source: PerceptionSource::DirectObservation,
        }
    }

    #[test]
    fn test_activation_computation_single_observation() {
        let current_tick = Tick(1000);
        let reference_cases = [
            (Tick(999), 1000),
            (Tick(996), 500),
            (Tick(984), 250),
            (Tick(952), 144),
            (Tick(900), 100),
            (Tick(600), 50),
        ];

        for (observed_tick, expected) in reference_cases {
            assert_eq!(
                compute_activation(current_tick, &[observed_tick], 1),
                expected,
                "age {}",
                current_tick.0 - observed_tick.0
            );
        }
    }

    #[test]
    fn test_activation_computation_multiple_observations() {
        let current_tick = Tick(50);
        let presentation_ticks = [Tick(45), Tick(35), Tick(25), Tick(15), Tick(5)];

        assert_eq!(
            compute_activation(current_tick, &presentation_ticks, 5),
            1223
        );
    }

    #[test]
    fn test_activation_computation_empty_buffer() {
        assert_eq!(compute_activation(Tick(10), &[], 0), 0);
    }

    #[test]
    fn test_salience_boost_scales_with_need_urgency() {
        let threshold = Permille::new(500).unwrap();
        let boost = Permille::new(500).unwrap();

        assert_eq!(
            salience_boost(500, Some(EntityKind::ItemLot), threshold, boost),
            250
        );
        assert_eq!(
            salience_boost(1000, Some(EntityKind::ItemLot), threshold, boost),
            500
        );
    }

    #[test]
    fn test_salience_boost_zero_below_threshold() {
        assert_eq!(
            salience_boost(
                499,
                Some(EntityKind::ItemLot),
                Permille::new(500).unwrap(),
                Permille::new(500).unwrap(),
            ),
            0
        );
    }

    #[test]
    fn test_salience_boost_zero_for_non_items() {
        assert_eq!(
            salience_boost(
                1000,
                Some(EntityKind::Agent),
                Permille::new(500).unwrap(),
                Permille::new(500).unwrap(),
            ),
            0
        );
    }

    fn tell_profile() -> TellProfile {
        TellProfile {
            max_tell_candidates: 3,
            max_relay_chain_len: 2,
            conversation_memory_capacity: 2,
            conversation_memory_retention_ticks: 5,
        }
    }

    fn tell_memory_key(counterparty: u32, subject: u32) -> TellMemoryKey {
        TellMemoryKey {
            counterparty: entity(counterparty),
            topic: TellTopic::EntityBelief {
                subject: entity(subject),
            },
        }
    }

    fn told_memory(
        counterparty: u32,
        subject: u32,
        told_tick: u64,
        state: &BelievedEntityState,
    ) -> (TellMemoryKey, ToldBeliefMemory) {
        (
            tell_memory_key(counterparty, subject),
            ToldBeliefMemory {
                shared_state: SharedTellState::EntityBelief(to_shared_belief_snapshot(state)),
                told_tick: Tick(told_tick),
            },
        )
    }

    fn heard_memory(
        counterparty: u32,
        subject: u32,
        heard_tick: u64,
        state: &BelievedEntityState,
        disposition: HeardBeliefDisposition,
    ) -> (TellMemoryKey, HeardBeliefMemory) {
        (
            tell_memory_key(counterparty, subject),
            HeardBeliefMemory {
                heard_state: SharedTellState::EntityBelief(to_shared_belief_snapshot(state)),
                heard_tick: Tick(heard_tick),
                disposition,
            },
        )
    }

    fn ask_memory_key(
        counterparty: u32,
        topic_entity: Option<u32>,
        topic_commodity: Option<CommodityKind>,
    ) -> AskWitnessMemoryKey {
        AskWitnessMemoryKey {
            counterparty: entity(counterparty),
            topic_entity: topic_entity.map(entity),
            topic_commodity,
        }
    }

    fn office_holder_belief(
        office: u32,
        holder: Option<u32>,
        source: InstitutionalKnowledgeSource,
        learned_tick: u64,
    ) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::OfficeHolder {
                office: entity(office),
                holder: holder.map(entity),
                effective_tick: Tick(learned_tick),
            },
            source,
            learned_tick: Tick(learned_tick),
            learned_at: Some(entity(9)),
        }
    }

    fn accusation_belief(
        accused: u32,
        violation_id: u64,
        learned_tick: u64,
    ) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::Accusation {
                accuser: entity(70),
                accused: entity(accused),
                violation_id: crate::ViolationId(violation_id),
                theft: TheftFacts {
                    missing_entity: entity(74),
                    expected_place: entity(75),
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(2),
                },
                effective_tick: Tick(learned_tick),
            },
            source: InstitutionalKnowledgeSource::RecordConsultation {
                record: entity(71),
                entry_id: crate::RecordEntryId(1),
            },
            learned_tick: Tick(learned_tick),
            learned_at: Some(entity(72)),
        }
    }

    fn verdict_belief(
        accused: u32,
        violation_id: u64,
        learned_tick: u64,
    ) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::Verdict {
                accused: entity(accused),
                violation_id: crate::ViolationId(violation_id),
                punishment: crate::PunishmentKind::Exile {
                    from_faction: entity(73),
                },
                effective_tick: Tick(learned_tick),
            },
            source: InstitutionalKnowledgeSource::RecordConsultation {
                record: entity(71),
                entry_id: crate::RecordEntryId(2),
            },
            learned_tick: Tick(learned_tick),
            learned_at: Some(entity(72)),
        }
    }

    fn membership_belief(
        faction: u32,
        member: u32,
        active: bool,
        learned_tick: u64,
    ) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::FactionMembership {
                faction: entity(faction),
                member: entity(member),
                active,
                effective_tick: Tick(learned_tick),
            },
            source: InstitutionalKnowledgeSource::WitnessedEvent,
            learned_tick: Tick(learned_tick),
            learned_at: Some(entity(11)),
        }
    }

    fn rally_point_belief(
        faction: u32,
        rally_place: Option<u32>,
        learned_tick: u64,
    ) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::FactionRallyPoint {
                faction: entity(faction),
                rally_place: rally_place.map(entity),
                effective_tick: Tick(learned_tick),
            },
            source: InstitutionalKnowledgeSource::DirectObservation,
            learned_tick: Tick(learned_tick),
            learned_at: Some(entity(15)),
        }
    }

    fn support_belief(
        office: u32,
        supporter: u32,
        candidate: Option<u32>,
        learned_tick: u64,
    ) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::SupportDeclaration {
                office: entity(office),
                supporter: entity(supporter),
                candidate: candidate.map(entity),
                effective_tick: Tick(learned_tick),
            },
            source: InstitutionalKnowledgeSource::RecordConsultation {
                record: entity(12),
                entry_id: crate::RecordEntryId(learned_tick),
            },
            learned_tick: Tick(learned_tick),
            learned_at: Some(entity(13)),
        }
    }

    fn force_control_belief(
        office: u32,
        controller: Option<u32>,
        contested: bool,
        learned_tick: u64,
    ) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::ForceControl {
                office: entity(office),
                controller: controller.map(entity),
                contested,
                effective_tick: Tick(learned_tick),
            },
            source: InstitutionalKnowledgeSource::WitnessedEvent,
            learned_tick: Tick(learned_tick),
            learned_at: Some(entity(14)),
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_serde_bounds<T: Eq + Clone + Serialize + DeserializeOwned>() {}

    fn assert_ordered_traits<T: Copy + Eq + Ord + std::hash::Hash>() {}

    #[test]
    fn new_creates_empty_store() {
        let store = AgentBeliefStore::new();

        assert!(store.entity_claims.is_empty());
        assert_eq!(store.next_claim_id, ClaimId(0));
        assert!(store.known_entities.is_empty());
        assert!(store.social_observations.is_empty());
        assert!(store.told_beliefs.is_empty());
        assert!(store.heard_beliefs.is_empty());
        assert!(store.asked_witnesses.is_empty());
        assert!(store.place_visits.is_empty());
        assert!(store.institutional_beliefs.is_empty());
    }

    #[test]
    fn record_place_visit_inserts_new_place_with_arrival_tick() {
        let mut store = AgentBeliefStore::new();
        let place = entity(22);

        assert!(store.record_place_visit(place, Tick(7)));
        assert_eq!(
            store.place_visits.get(&place),
            Some(&PlaceVisitRecord {
                ticks_present: 0,
                last_arrival_tick: Tick(7),
                visit_count: 1,
            })
        );
    }

    #[test]
    fn record_place_visit_increments_presence_on_contiguous_tick() {
        let mut store = AgentBeliefStore::new();
        let place = entity(23);

        assert!(store.record_place_visit(place, Tick(7)));
        assert!(store.record_place_visit(place, Tick(8)));
        assert!(store.record_place_visit(place, Tick(9)));
        assert_eq!(
            store.place_visits.get(&place),
            Some(&PlaceVisitRecord {
                ticks_present: 2,
                last_arrival_tick: Tick(7),
                visit_count: 1,
            })
        );
    }

    #[test]
    fn record_place_visit_resets_presence_and_increments_visit_count_on_return() {
        let mut store = AgentBeliefStore::new();
        let place = entity(24);

        assert!(store.record_place_visit(place, Tick(7)));
        assert!(store.record_place_visit(place, Tick(8)));
        assert!(store.record_place_visit(place, Tick(12)));
        assert_eq!(
            store.place_visits.get(&place),
            Some(&PlaceVisitRecord {
                ticks_present: 0,
                last_arrival_tick: Tick(12),
                visit_count: 2,
            })
        );
    }

    #[test]
    fn derive_entity_summary_returns_none_for_empty_claims() {
        assert_eq!(derive_entity_summary(&[], Tick(10), &policy()), None);
    }

    #[test]
    fn derive_entity_summary_projects_single_claims_into_summary() {
        let claims = vec![
            sample_claim(
                1,
                40,
                EntityBeliefAspect::Location,
                ClaimValue::Place(Some(entity(10))),
                PerceptionSource::DirectObservation,
                7,
                950,
            ),
            sample_claim(
                2,
                40,
                EntityBeliefAspect::Alive,
                ClaimValue::Bool(true),
                PerceptionSource::DirectObservation,
                7,
                950,
            ),
            sample_claim(
                3,
                40,
                EntityBeliefAspect::Inventory(CommodityKind::Apple),
                ClaimValue::Quantity(Quantity(4)),
                PerceptionSource::DirectObservation,
                7,
                950,
            ),
            sample_claim(
                4,
                40,
                EntityBeliefAspect::Wounded,
                ClaimValue::WoundSnapshot(vec![sample_wound(2, 7)]),
                PerceptionSource::DirectObservation,
                7,
                950,
            ),
            sample_claim(
                5,
                40,
                EntityBeliefAspect::WorkstationPresent,
                ClaimValue::WorkstationTag(Some(WorkstationTag::Mill)),
                PerceptionSource::DirectObservation,
                7,
                950,
            ),
            sample_claim(
                6,
                40,
                EntityBeliefAspect::ResourceAvailable(CommodityKind::Apple),
                ClaimValue::ResourceSource(Some(ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(5),
                    max_quantity: Quantity(9),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: Some(Tick(6)),
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                })),
                PerceptionSource::DirectObservation,
                7,
                950,
            ),
            sample_claim(
                7,
                40,
                EntityBeliefAspect::WashBasinState,
                ClaimValue::WashBasinState(Some(WashBasinState {
                    clean_water_units: 8,
                    units_per_full_wash: 2,
                    dirtiness_level: Permille::new(300).unwrap(),
                    ..WashBasinState::default()
                })),
                PerceptionSource::DirectObservation,
                7,
                950,
            ),
        ];

        let summary = derive_entity_summary(&claims, Tick(10), &policy()).unwrap();

        assert_eq!(summary.last_known_place, Some(entity(10)));
        assert_eq!(
            summary.last_known_inventory.get(&CommodityKind::Apple),
            Some(&Quantity(4))
        );
        assert!(summary.alive);
        assert_eq!(summary.wounds, vec![sample_wound(2, 7)]);
        assert_eq!(summary.workstation_tag, Some(WorkstationTag::Mill));
        assert_eq!(
            summary.resource_source,
            Some(ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(5),
                max_quantity: Quantity(9),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: Some(Tick(6)),
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            })
        );
        assert_eq!(
            summary.wash_basin_state,
            Some(WashBasinState {
                clean_water_units: 8,
                units_per_full_wash: 2,
                dirtiness_level: Permille::new(300).unwrap(),
                ..WashBasinState::default()
            })
        );
        assert_eq!(summary.last_observed_tick(), Some(Tick(7)));
        assert_eq!(summary.source, PerceptionSource::DirectObservation);
    }

    #[test]
    fn record_entity_snapshot_claims_preserves_wash_basin_state_in_summary() {
        let subject = entity(40);
        let basin_state = WashBasinState {
            clean_water_units: 7,
            units_per_full_wash: 3,
            dirtiness_level: Permille::new(250).unwrap(),
            ..WashBasinState::default()
        };
        let snapshot = BelievedEntityState {
            believed_kind: Some(EntityKind::Facility),
            last_known_place: Some(entity(10)),
            workstation_tag: Some(WorkstationTag::WashBasin),
            wash_basin_state: Some(basin_state),
            ..BelievedEntityState::single_observation_defaults(
                Tick(12),
                PerceptionSource::DirectObservation,
            )
        };
        let mut store = AgentBeliefStore::new();

        store.record_entity_snapshot_claims(
            subject,
            &snapshot,
            None,
            Tick(12),
            Some(Tick(12)),
            8,
            &policy(),
        );

        let summary = store
            .get_entity(&subject)
            .expect("claim-backed summary should exist");
        assert_eq!(summary.wash_basin_state, Some(basin_state));
        assert!(
            store
                .get_entity_claims(&subject)
                .expect("claims should exist")
                .iter()
                .any(|claim| claim.aspect == EntityBeliefAspect::WashBasinState
                    && claim.value == ClaimValue::WashBasinState(Some(basin_state)))
        );
    }

    #[test]
    fn derive_entity_summary_prefers_highest_effective_confidence_per_aspect() {
        let claims = vec![
            sample_claim(
                1,
                41,
                EntityBeliefAspect::Location,
                ClaimValue::Place(Some(entity(10))),
                PerceptionSource::Report {
                    from: entity(2),
                    chain_len: 1,
                },
                7,
                650,
            ),
            sample_claim(
                2,
                41,
                EntityBeliefAspect::Location,
                ClaimValue::Place(Some(entity(11))),
                PerceptionSource::DirectObservation,
                7,
                900,
            ),
        ];

        let summary = derive_entity_summary(&claims, Tick(10), &policy()).unwrap();

        assert_eq!(summary.last_known_place, Some(entity(11)));
        assert_eq!(summary.last_observed_tick(), Some(Tick(7)));
        assert_eq!(summary.source, PerceptionSource::DirectObservation);
    }

    #[test]
    fn derive_entity_summary_applies_staleness_before_selecting_winner() {
        let claims = vec![
            sample_claim(
                1,
                42,
                EntityBeliefAspect::Location,
                ClaimValue::Place(Some(entity(10))),
                PerceptionSource::DirectObservation,
                1,
                900,
            ),
            sample_claim(
                2,
                42,
                EntityBeliefAspect::Location,
                ClaimValue::Place(Some(entity(11))),
                PerceptionSource::Report {
                    from: entity(3),
                    chain_len: 1,
                },
                9,
                820,
            ),
        ];

        let summary = derive_entity_summary(&claims, Tick(10), &policy()).unwrap();

        assert_eq!(summary.last_known_place, Some(entity(11)));
        assert_eq!(
            summary.source,
            PerceptionSource::Report {
                from: entity(3),
                chain_len: 1
            }
        );
    }

    #[test]
    fn derive_entity_summary_uses_claimed_event_tick_for_report_staleness() {
        let claims = vec![
            EntityBeliefClaim {
                claim_id: ClaimId(1),
                subject: entity(44),
                aspect: EntityBeliefAspect::Location,
                value: ClaimValue::Place(Some(entity(10))),
                source: PerceptionSource::Report {
                    from: entity(3),
                    chain_len: 1,
                },
                acquired_tick: Tick(9),
                claimed_event_tick: Some(Tick(1)),
                confidence: Permille::new(900).unwrap(),
                refuted_at_tick: None,
            },
            EntityBeliefClaim {
                claim_id: ClaimId(2),
                subject: entity(44),
                aspect: EntityBeliefAspect::Location,
                value: ClaimValue::Place(Some(entity(11))),
                source: PerceptionSource::DirectObservation,
                acquired_tick: Tick(7),
                claimed_event_tick: Some(Tick(7)),
                confidence: Permille::new(830).unwrap(),
                refuted_at_tick: None,
            },
        ];

        let summary = derive_entity_summary(&claims, Tick(10), &policy()).unwrap();

        assert_eq!(summary.last_known_place, Some(entity(11)));
        assert_eq!(summary.source, PerceptionSource::DirectObservation);
    }

    #[test]
    fn derive_entity_summary_ignores_refuted_claims() {
        let claims = vec![
            EntityBeliefClaim {
                refuted_at_tick: Some(Tick(9)),
                ..sample_claim(
                    1,
                    240,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(10))),
                    PerceptionSource::DirectObservation,
                    7,
                    980,
                )
            },
            sample_claim(
                2,
                240,
                EntityBeliefAspect::Location,
                ClaimValue::Place(Some(entity(11))),
                PerceptionSource::Report {
                    from: entity(3),
                    chain_len: 1,
                },
                8,
                780,
            ),
            sample_claim(
                3,
                240,
                EntityBeliefAspect::Alive,
                ClaimValue::Bool(true),
                PerceptionSource::DirectObservation,
                8,
                950,
            ),
        ];

        let summary = derive_entity_summary(&claims, Tick(10), &policy()).unwrap();

        assert_eq!(summary.last_known_place, Some(entity(11)));
        assert_eq!(summary.source, PerceptionSource::DirectObservation);
    }

    #[test]
    fn record_entity_claim_refutes_conflicting_direct_claims_on_same_aspect() {
        let subject = entity(44);
        let mut store = AgentBeliefStore::new();

        store.record_entity_claim(sample_claim(
            1,
            subject.slot,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(entity(10))),
            PerceptionSource::DirectObservation,
            7,
            950,
        ));
        store.record_entity_claim(sample_claim(
            2,
            subject.slot,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(entity(11))),
            PerceptionSource::DirectObservation,
            8,
            950,
        ));

        let claims = &store.entity_claims[&subject];
        assert_eq!(claims.len(), 2);
        assert_eq!(claims[0].value, ClaimValue::Place(Some(entity(10))));
        assert_eq!(claims[0].refuted_at_tick, Some(Tick(8)));
        assert_eq!(claims[1].value, ClaimValue::Place(Some(entity(11))));
        assert_eq!(claims[1].refuted_at_tick, None);

        store.refresh_entity_summary_from_claims(subject, Tick(8), &policy());
        assert_eq!(
            store.get_entity(&subject).unwrap().last_known_place,
            Some(entity(11))
        );
    }

    #[test]
    fn record_entity_claim_replaces_dominated_same_value_direct_claims() {
        let subject = entity(144);
        let mut store = AgentBeliefStore::new();

        store.record_entity_claim(sample_claim(
            1,
            subject.slot,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(entity(10))),
            PerceptionSource::DirectObservation,
            7,
            950,
        ));
        store.record_entity_claim(sample_claim(
            2,
            subject.slot,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(entity(10))),
            PerceptionSource::DirectObservation,
            8,
            950,
        ));

        let claims = &store.entity_claims[&subject];
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].value, ClaimValue::Place(Some(entity(10))));
        assert_eq!(claims[0].acquired_tick, Tick(8));
        assert_eq!(claims[0].refuted_at_tick, None);
    }

    #[test]
    fn record_entity_claim_preserves_different_sources() {
        let subject = entity(45);
        let mut store = AgentBeliefStore::new();

        store.record_entity_claim(sample_claim(
            1,
            subject.slot,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(entity(10))),
            PerceptionSource::DirectObservation,
            7,
            950,
        ));
        store.record_entity_claim(sample_claim(
            2,
            subject.slot,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(entity(11))),
            PerceptionSource::Report {
                from: entity(2),
                chain_len: 1,
            },
            8,
            780,
        ));

        assert_eq!(store.entity_claims[&subject].len(), 2);
    }

    #[test]
    fn record_entity_claim_distinguishes_report_informants() {
        let subject = entity(46);
        let mut store = AgentBeliefStore::new();

        store.record_entity_claim(sample_claim(
            1,
            subject.slot,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(entity(10))),
            PerceptionSource::Report {
                from: entity(2),
                chain_len: 1,
            },
            7,
            780,
        ));
        store.record_entity_claim(sample_claim(
            2,
            subject.slot,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(entity(11))),
            PerceptionSource::Report {
                from: entity(3),
                chain_len: 1,
            },
            8,
            780,
        ));

        assert_eq!(store.entity_claims[&subject].len(), 2);
    }

    #[test]
    fn record_entity_claim_preserves_nondominated_same_informant_report() {
        let subject = entity(47);
        let mut store = AgentBeliefStore::new();

        store.record_entity_claim(EntityBeliefClaim {
            claim_id: ClaimId(1),
            subject,
            aspect: EntityBeliefAspect::Location,
            value: ClaimValue::Place(Some(entity(10))),
            source: PerceptionSource::Report {
                from: entity(2),
                chain_len: 1,
            },
            acquired_tick: Tick(8),
            claimed_event_tick: Some(Tick(8)),
            confidence: Permille::new(780).unwrap(),
            refuted_at_tick: None,
        });
        store.record_entity_claim(EntityBeliefClaim {
            claim_id: ClaimId(2),
            subject,
            aspect: EntityBeliefAspect::Location,
            value: ClaimValue::Place(Some(entity(11))),
            source: PerceptionSource::Report {
                from: entity(2),
                chain_len: 3,
            },
            acquired_tick: Tick(9),
            claimed_event_tick: Some(Tick(2)),
            confidence: Permille::new(600).unwrap(),
            refuted_at_tick: None,
        });

        let claims = &store.entity_claims[&subject];
        assert_eq!(claims.len(), 2);
        assert!(claims.iter().any(|claim| {
            claim.value == ClaimValue::Place(Some(entity(10)))
                && claim.claimed_event_tick == Some(Tick(8))
        }));
        assert!(claims.iter().any(|claim| {
            claim.value == ClaimValue::Place(Some(entity(11)))
                && claim.claimed_event_tick == Some(Tick(2))
        }));
    }

    #[test]
    fn record_entity_snapshot_claims_derives_summary_and_clears_absent_inventory() {
        let subject = entity(42);
        let mut store = AgentBeliefStore::new();
        let mut prior_inventory = BTreeMap::new();
        prior_inventory.insert(CommodityKind::Apple, Quantity(4));
        prior_inventory.insert(CommodityKind::Bread, Quantity(2));
        let prior = BelievedEntityState {
            believed_kind: Some(EntityKind::Place),
            last_known_place: Some(entity(10)),
            last_known_inventory: prior_inventory,
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                Tick(3),
                PerceptionSource::DirectObservation,
            )
        };
        store.update_entity(subject, prior.clone());

        let mut snapshot_inventory = BTreeMap::new();
        snapshot_inventory.insert(CommodityKind::Bread, Quantity(5));
        let snapshot = BelievedEntityState {
            believed_kind: Some(EntityKind::Place),
            last_known_place: Some(entity(11)),
            last_known_inventory: snapshot_inventory,
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                Tick(8),
                PerceptionSource::DirectObservation,
            )
        };

        store.record_entity_snapshot_claims(
            subject,
            &snapshot,
            Some(&prior),
            Tick(8),
            Some(Tick(8)),
            5,
            &policy(),
        );

        let summary = store.get_entity(&subject).unwrap();
        assert_eq!(summary.last_known_place, Some(entity(11)));
        assert_eq!(
            summary.last_known_inventory.get(&CommodityKind::Bread),
            Some(&Quantity(5))
        );
        assert!(
            !summary
                .last_known_inventory
                .contains_key(&CommodityKind::Apple)
        );
        assert_eq!(store.next_claim_id, ClaimId(3));
        assert!(store.entity_claims[&subject].iter().any(|claim| {
            claim.aspect == EntityBeliefAspect::Inventory(CommodityKind::Apple)
                && claim.value == ClaimValue::Quantity(Quantity(0))
        }));
    }

    #[test]
    fn record_entity_snapshot_claims_clears_prior_resource_lane() {
        let subject = entity(43);
        let mut store = AgentBeliefStore::new();
        let prior = BelievedEntityState {
            believed_kind: Some(EntityKind::Facility),
            last_known_place: Some(entity(10)),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: Some(ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(3),
                max_quantity: Quantity(7),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: Some(Tick(2)),
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            }),
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                Tick(2),
                PerceptionSource::DirectObservation,
            )
        };
        store.update_entity(subject, prior.clone());

        let snapshot = BelievedEntityState {
            believed_kind: Some(EntityKind::Facility),
            last_known_place: Some(entity(10)),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                Tick(6),
                PerceptionSource::DirectObservation,
            )
        };

        store.record_entity_snapshot_claims(
            subject,
            &snapshot,
            Some(&prior),
            Tick(6),
            Some(Tick(6)),
            5,
            &policy(),
        );

        let summary = store.get_entity(&subject).unwrap();
        assert_eq!(summary.resource_source, None);
        assert!(store.entity_claims[&subject].iter().any(|claim| {
            claim.aspect == EntityBeliefAspect::ResourceAvailable(CommodityKind::Apple)
                && claim.value == ClaimValue::ResourceSource(None)
        }));
    }

    #[test]
    fn record_entity_snapshot_claims_omits_empty_baseline_aspects_without_prior_state() {
        let subject = entity(44);
        let mut store = AgentBeliefStore::new();
        let snapshot = BelievedEntityState {
            believed_kind: Some(EntityKind::ItemLot),
            last_known_place: Some(entity(10)),
            last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(2))]),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                Tick(5),
                PerceptionSource::DirectObservation,
            )
        };

        store.record_entity_snapshot_claims(
            subject,
            &snapshot,
            None,
            Tick(5),
            Some(Tick(5)),
            5,
            &policy(),
        );

        let claims = &store.entity_claims[&subject];
        assert_eq!(claims.len(), 3);
        assert!(claims.iter().any(|claim| {
            claim.aspect == EntityBeliefAspect::Location
                && claim.value == ClaimValue::Place(Some(entity(10)))
        }));
        assert!(claims.iter().any(|claim| {
            claim.aspect == EntityBeliefAspect::Alive && claim.value == ClaimValue::Bool(true)
        }));
        assert!(claims.iter().any(|claim| {
            claim.aspect == EntityBeliefAspect::Inventory(CommodityKind::Bread)
                && claim.value == ClaimValue::Quantity(Quantity(2))
        }));
    }

    #[test]
    fn record_entity_snapshot_claims_emits_alive_false_to_clear_prior_living_summary() {
        let subject = entity(45);
        let mut store = AgentBeliefStore::new();
        let prior = BelievedEntityState {
            believed_kind: Some(EntityKind::Agent),
            last_known_place: Some(entity(10)),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                Tick(4),
                PerceptionSource::DirectObservation,
            )
        };
        let snapshot = BelievedEntityState {
            alive: false,
            presentation_ticks: [
                Tick(5),
                Tick(0),
                Tick(0),
                Tick(0),
                Tick(0),
                Tick(0),
                Tick(0),
                Tick(0),
            ],
            presentation_tick_count: 1,
            ..prior.clone()
        };

        store.record_entity_snapshot_claims(
            subject,
            &snapshot,
            Some(&prior),
            Tick(5),
            Some(Tick(5)),
            5,
            &policy(),
        );

        assert!(store.entity_claims[&subject].iter().any(|claim| {
            claim.aspect == EntityBeliefAspect::Alive && claim.value == ClaimValue::Bool(false)
        }));
        assert!(!store.get_entity(&subject).unwrap().alive);
    }

    #[test]
    fn derive_entity_summary_breaks_ties_by_newer_tick_then_higher_claim_id() {
        let newer = sample_claim(
            2,
            43,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(entity(11))),
            PerceptionSource::Report {
                from: entity(4),
                chain_len: 1,
            },
            9,
            700,
        );
        let older = sample_claim(
            1,
            43,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(entity(10))),
            PerceptionSource::DirectObservation,
            7,
            700,
        );
        let tied_newer_low_id = sample_claim(
            3,
            43,
            EntityBeliefAspect::Alive,
            ClaimValue::Bool(true),
            PerceptionSource::DirectObservation,
            9,
            700,
        );
        let tied_newer_high_id = sample_claim(
            4,
            43,
            EntityBeliefAspect::Alive,
            ClaimValue::Bool(false),
            PerceptionSource::DirectObservation,
            9,
            700,
        );

        let summary = derive_entity_summary(
            &[older, newer, tied_newer_low_id, tied_newer_high_id],
            Tick(9),
            &policy(),
        )
        .unwrap();

        assert_eq!(summary.last_known_place, Some(entity(11)));
        assert!(!summary.alive);
    }

    #[test]
    fn update_entity_inserts_new_snapshot() {
        let mut store = AgentBeliefStore::new();
        let target = entity(3);
        let state = sample_state(7, 4);

        store.update_entity(target, state.clone());

        assert_eq!(store.get_entity(&target), Some(&state));
    }

    #[test]
    fn update_entity_replaces_with_equal_or_newer_snapshot_only() {
        let mut store = AgentBeliefStore::new();
        let target = entity(4);

        store.update_entity(target, sample_state(8, 2));
        store.update_entity(target, sample_state(7, 9));
        assert_eq!(
            store.get_entity(&target).unwrap().last_known_inventory[&CommodityKind::Apple],
            Quantity(2)
        );

        store.update_entity(target, sample_state(8, 5));
        assert_eq!(
            store.get_entity(&target).unwrap().last_known_inventory[&CommodityKind::Apple],
            Quantity(5)
        );

        store.update_entity(target, sample_state(9, 6));
        assert_eq!(
            store.get_entity(&target).unwrap().last_known_inventory[&CommodityKind::Apple],
            Quantity(6)
        );
    }

    #[test]
    fn get_entity_returns_none_for_unknown_entity() {
        let store = AgentBeliefStore::new();

        assert_eq!(store.get_entity(&entity(404)), None);
    }

    #[test]
    fn record_social_observation_keeps_distinct_details() {
        let mut store = AgentBeliefStore::new();
        let first = sample_social_observation(3);
        let second = SocialObservation {
            detail: SocialObservationDetail::CoPresence { other: entity(7) },
            ..sample_social_observation(4)
        };

        store.record_social_observation(first);
        store.record_social_observation(second);

        assert_eq!(store.social_observations, vec![first, second]);
    }

    #[test]
    fn record_social_observation_replaces_same_detail_with_newer_observation() {
        let mut store = AgentBeliefStore::new();
        let first = sample_social_observation(3);
        let newer = SocialObservation {
            place: entity(55),
            observed_tick: Tick(6),
            source: PerceptionSource::Report {
                from: entity(9),
                chain_len: 1,
            },
            ..first
        };

        store.record_social_observation(first);
        store.record_social_observation(newer);

        assert_eq!(store.social_observations, vec![newer]);
    }

    #[test]
    fn record_social_observation_keeps_newer_same_detail_when_older_arrives() {
        let mut store = AgentBeliefStore::new();
        let newer = sample_social_observation(6);
        let older = SocialObservation {
            place: entity(55),
            observed_tick: Tick(3),
            source: PerceptionSource::Report {
                from: entity(9),
                chain_len: 1,
            },
            ..newer
        };

        store.record_social_observation(newer);
        store.record_social_observation(older);

        assert_eq!(store.social_observations, vec![newer]);
    }

    #[test]
    fn relayable_social_observations_exclude_witnessed_telling_feedback() {
        let mut store = AgentBeliefStore::new();
        let relayable = sample_social_observation(3);
        let witnessed_telling = SocialObservation {
            detail: SocialObservationDetail::WitnessedTelling {
                speaker: entity(7),
                listener: entity(8),
            },
            place: entity(10),
            observed_tick: Tick(4),
            source: PerceptionSource::DirectObservation,
        };

        store.record_social_observation(relayable);
        store.record_social_observation(witnessed_telling);

        assert_eq!(store.relayable_social_observations(2), vec![relayable]);
    }

    #[test]
    fn shared_tell_state_for_topic_rejects_witnessed_telling_topics() {
        let mut store = AgentBeliefStore::new();
        let witnessed_telling = SocialObservation {
            detail: SocialObservationDetail::WitnessedTelling {
                speaker: entity(7),
                listener: entity(8),
            },
            place: entity(10),
            observed_tick: Tick(4),
            source: PerceptionSource::DirectObservation,
        };
        store.record_social_observation(witnessed_telling);

        assert_eq!(
            store.shared_tell_state_for_topic(
                &TellTopic::SocialObservation {
                    observation: witnessed_telling,
                },
                2,
            ),
            None
        );
    }

    /*
    #[test]
    fn enforce_capacity_evicts_oldest_entities_deterministically() {
        let mut store = AgentBeliefStore::new();
        store.update_entity(entity(3), sample_state(5, 1));
        store.update_entity(entity(2), sample_state(5, 2));
        store.update_entity(entity(4), sample_state(6, 3));

        store.enforce_capacity(&profile(2, 8, 100), Tick(20));

        assert_eq!(store.known_entities.len(), 2);
        assert!(!store.known_entities.contains_key(&entity(2)));
        assert!(store.known_entities.contains_key(&entity(3)));
        assert!(store.known_entities.contains_key(&entity(4)));
    }

    #[test]
    fn enforce_capacity_removes_stale_entities_and_social_observations() {
        let mut store = AgentBeliefStore::new();
        store.update_entity(entity(1), sample_state(2, 1));
        store.update_entity(entity(2), sample_state(9, 2));
        store.record_social_observation(sample_social_observation(3));
        store.record_social_observation(sample_social_observation(9));

        store.enforce_capacity(&profile(10, 10, 3), Tick(12));

        assert!(!store.known_entities.contains_key(&entity(1)));
        assert!(store.known_entities.contains_key(&entity(2)));
        assert_eq!(
            store.social_observations,
            vec![sample_social_observation(9)]
        );
    }

    #[test]
    fn enforce_capacity_clears_entities_when_capacity_is_zero() {
        let mut store = AgentBeliefStore::new();
        store.update_entity(entity(1), sample_state(10, 1));
        store.update_entity(entity(2), sample_state(11, 2));

        store.enforce_capacity(&profile(0, 5, 100), Tick(12));

        assert!(store.known_entities.is_empty());
    }

    #[test]
    fn enforce_capacity_applies_global_entity_cap_after_claim_pruning() {
        let mut store = claim_backed_store(
            80,
            vec![sample_claim(
                1,
                80,
                EntityBeliefAspect::Alive,
                ClaimValue::Bool(true),
                PerceptionSource::DirectObservation,
                5,
                950,
            )],
        );
        store.update_entity(entity(1), sample_state(9, 1));

        store.enforce_capacity(&profile(1, 8, 100), Tick(12));

        assert_eq!(store.known_entities.len(), 1);
        assert!(store.known_entities.contains_key(&entity(1)));
        assert!(!store.known_entities.contains_key(&entity(80)));
        assert!(!store.entity_claims.contains_key(&entity(80)));
    }

    #[test]
    fn enforce_capacity_preserves_infrastructure_entities() {
        let mut store = AgentBeliefStore::new();

        let mut place_a = sample_state(3, 0);
        place_a.believed_kind = Some(EntityKind::Place);
        let mut place_b = sample_state(4, 0);
        place_b.believed_kind = Some(EntityKind::Place);
        let mut item_a = sample_state(1, 1);
        item_a.believed_kind = Some(EntityKind::ItemLot);
        let mut item_b = sample_state(2, 2);
        item_b.believed_kind = Some(EntityKind::ItemLot);
        let mut item_c = sample_state(5, 3);
        item_c.believed_kind = Some(EntityKind::ItemLot);

        store.update_entity(entity(101), place_a);
        store.update_entity(entity(102), place_b);
        store.update_entity(entity(201), item_a);
        store.update_entity(entity(202), item_b);
        store.update_entity(entity(203), item_c);

        store.enforce_capacity(&profile(3, 8, 100), Tick(20));

        assert!(store.known_entities.contains_key(&entity(101)));
        assert!(store.known_entities.contains_key(&entity(102)));
        assert!(!store.known_entities.contains_key(&entity(201)));
        assert!(!store.known_entities.contains_key(&entity(202)));
        assert!(store.known_entities.contains_key(&entity(203)));
    }

    #[test]
    fn enforce_capacity_resource_source_override_promotes_to_infrastructure() {
        let mut store = AgentBeliefStore::new();

        let mut resource_item = sample_state(1, 1);
        resource_item.believed_kind = Some(EntityKind::ItemLot);
        resource_item.resource_source = Some(ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(2),
            max_quantity: Quantity(5),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
        });
        let mut transient_item = sample_state(2, 1);
        transient_item.believed_kind = Some(EntityKind::ItemLot);

        store.update_entity(entity(301), resource_item);
        store.update_entity(entity(302), transient_item);

        store.enforce_capacity(&profile(1, 8, 100), Tick(20));

        assert!(store.known_entities.contains_key(&entity(301)));
        assert!(!store.known_entities.contains_key(&entity(302)));
    }

    #[test]
    fn enforce_capacity_dead_agents_are_transient() {
        let mut store = AgentBeliefStore::new();

        let mut dead_agent = sample_state(1, 1);
        dead_agent.believed_kind = Some(EntityKind::Agent);
        dead_agent.alive = false;
        let mut live_agent = sample_state(2, 1);
        live_agent.believed_kind = Some(EntityKind::Agent);

        store.update_entity(entity(401), dead_agent);
        store.update_entity(entity(402), live_agent);

        store.enforce_capacity(&profile(1, 8, 100), Tick(20));

        assert!(!store.known_entities.contains_key(&entity(401)));
        assert!(store.known_entities.contains_key(&entity(402)));
    }

    #[test]
    fn enforce_capacity_unknown_kind_is_transient() {
        let mut store = AgentBeliefStore::new();

        let mut unknown = sample_state(1, 1);
        unknown.believed_kind = None;
        let mut facility = sample_state(2, 1);
        facility.believed_kind = Some(EntityKind::Facility);

        store.update_entity(entity(501), unknown);
        store.update_entity(entity(502), facility);

        store.enforce_capacity(&profile(1, 8, 100), Tick(20));

        assert!(!store.known_entities.contains_key(&entity(501)));
        assert!(store.known_entities.contains_key(&entity(502)));
    }

    #[test]
    fn infrastructure_retention_entities_survive_longer() {
        let mut store = AgentBeliefStore::new();

        let mut place = sample_state(0, 1);
        place.believed_kind = Some(EntityKind::Place);
        let mut item = sample_state(0, 2);
        item.believed_kind = Some(EntityKind::ItemLot);

        store.update_entity(entity(601), place);
        store.update_entity(entity(602), item);

        let mut perception = profile(10, 10, 48);
        perception.infrastructure_retention_ticks = 480;

        store.enforce_capacity(&perception, Tick(100));

        assert!(store.known_entities.contains_key(&entity(601)));
        assert!(!store.known_entities.contains_key(&entity(602)));
    }

    #[test]
    fn infrastructure_retention_eventually_decays() {
        let mut store = AgentBeliefStore::new();

        let mut place = sample_state(0, 1);
        place.believed_kind = Some(EntityKind::Place);
        store.update_entity(entity(603), place);

        let mut perception = profile(10, 10, 48);
        perception.infrastructure_retention_ticks = 480;

        store.enforce_capacity(&perception, Tick(481));

        assert!(!store.known_entities.contains_key(&entity(603)));
    }

    #[test]
    fn infrastructure_retention_equal_parameters_no_regression() {
        let mut store = AgentBeliefStore::new();

        let mut place = sample_state(0, 1);
        place.believed_kind = Some(EntityKind::Place);
        let mut item = sample_state(0, 2);
        item.believed_kind = Some(EntityKind::ItemLot);

        store.update_entity(entity(604), place);
        store.update_entity(entity(605), item);

        let mut perception = profile(10, 10, 48);
        perception.infrastructure_retention_ticks = 48;

        store.enforce_capacity(&perception, Tick(49));

        assert!(!store.known_entities.contains_key(&entity(604)));
        assert!(!store.known_entities.contains_key(&entity(605)));
    }

    #[test]
    fn enforce_entity_claim_capacity_evicts_claims_beyond_retention_ticks() {
        let mut store = claim_backed_store(
            60,
            vec![
                sample_claim(
                    1,
                    60,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(10))),
                    PerceptionSource::DirectObservation,
                    1,
                    950,
                ),
                sample_claim(
                    2,
                    60,
                    EntityBeliefAspect::Alive,
                    ClaimValue::Bool(true),
                    PerceptionSource::DirectObservation,
                    9,
                    950,
                ),
            ],
        );

        store.enforce_entity_claim_capacity(&profile(8, 5, 3), Tick(12));

        assert_eq!(store.entity_claims.get(&entity(60)).unwrap().len(), 1);
        assert_eq!(
            store.entity_claims.get(&entity(60)).unwrap()[0].claim_id,
            ClaimId(2)
        );
        assert!(store.known_entities.contains_key(&entity(60)));
    }

    #[test]
    fn enforce_entity_claim_capacity_evicts_lowest_confidence_and_rederives_summary() {
        let mut store = claim_backed_store(
            61,
            vec![
                sample_claim(
                    1,
                    61,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(10))),
                    PerceptionSource::DirectObservation,
                    9,
                    900,
                ),
                sample_claim(
                    2,
                    61,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(11))),
                    PerceptionSource::DirectObservation,
                    10,
                    950,
                ),
                sample_claim(
                    3,
                    61,
                    EntityBeliefAspect::Alive,
                    ClaimValue::Bool(true),
                    PerceptionSource::DirectObservation,
                    10,
                    950,
                ),
            ],
        );

        store.enforce_entity_claim_capacity(&profile(8, 2, 100), Tick(10));

        let claims = store.entity_claims.get(&entity(61)).unwrap();
        assert_eq!(claims.len(), 2);
        assert!(!claims.iter().any(|claim| claim.claim_id == ClaimId(1)));
        assert_eq!(
            store
                .known_entities
                .get(&entity(61))
                .unwrap()
                .last_known_place,
            Some(entity(11))
        );
    }

    #[test]
    fn enforce_entity_claim_capacity_preserves_facility_location_for_resource_sources() {
        let mut store = claim_backed_store(
            81,
            vec![
                sample_claim(
                    1,
                    81,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(10))),
                    PerceptionSource::DirectObservation,
                    9,
                    950,
                ),
                sample_claim(
                    2,
                    81,
                    EntityBeliefAspect::Alive,
                    ClaimValue::Bool(true),
                    PerceptionSource::DirectObservation,
                    9,
                    950,
                ),
                sample_claim(
                    3,
                    81,
                    EntityBeliefAspect::WorkstationPresent,
                    ClaimValue::WorkstationTag(Some(WorkstationTag::OrchardRow)),
                    PerceptionSource::DirectObservation,
                    9,
                    950,
                ),
                sample_claim(
                    4,
                    81,
                    EntityBeliefAspect::ResourceAvailable(CommodityKind::Apple),
                    ClaimValue::ResourceSource(Some(ResourceSource {
                        commodity: CommodityKind::Apple,
                        available_quantity: Quantity(2),
                        max_quantity: Quantity(5),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    })),
                    PerceptionSource::DirectObservation,
                    9,
                    950,
                ),
            ],
        );
        store.update_entity(
            entity(81),
            BelievedEntityState {
                believed_kind: Some(EntityKind::Facility),
                last_known_place: Some(entity(10)),
                last_known_inventory: BTreeMap::new(),
                workstation_tag: Some(WorkstationTag::OrchardRow),
                resource_source: Some(ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(2),
                    max_quantity: Quantity(5),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                }),
                alive: true,
                wounds: Vec::new(),
                last_known_courage: None,
                believed_activity: None,
                believed_artifact: None,
                believed_contention: None,
                believed_evidence: None,
                ..BelievedEntityState::single_observation_defaults(
                    Tick(9),
                    PerceptionSource::DirectObservation,
                )
            },
        );

        store.enforce_entity_claim_capacity(&profile(8, 3, 100), Tick(10));

        let summary = store.known_entities.get(&entity(81)).unwrap();
        assert_eq!(
            summary.last_known_place,
            Some(entity(10)),
            "resource-source facilities must retain location so place-scoped opportunity discovery remains lawful"
        );
        assert_eq!(summary.workstation_tag, Some(WorkstationTag::OrchardRow));
        assert!(summary.resource_source.is_some());
    }

    #[test]
    fn enforce_entity_claim_capacity_preserves_infrastructure_claims() {
        let mut store = claim_backed_store(
            81,
            vec![
                sample_claim(
                    1,
                    81,
                    EntityBeliefAspect::ResourceAvailable(CommodityKind::Apple),
                    ClaimValue::ResourceSource(Some(ResourceSource {
                        commodity: CommodityKind::Apple,
                        available_quantity: Quantity(2),
                        max_quantity: Quantity(5),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    })),
                    PerceptionSource::DirectObservation,
                    9,
                    950,
                ),
                sample_claim(
                    2,
                    81,
                    EntityBeliefAspect::ResourceAvailable(CommodityKind::Bread),
                    ClaimValue::ResourceSource(Some(ResourceSource {
                        commodity: CommodityKind::Bread,
                        available_quantity: Quantity(3),
                        max_quantity: Quantity(6),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    })),
                    PerceptionSource::DirectObservation,
                    8,
                    920,
                ),
                sample_claim(
                    3,
                    81,
                    EntityBeliefAspect::Inventory(CommodityKind::Waste),
                    ClaimValue::Quantity(Quantity(4)),
                    PerceptionSource::DirectObservation,
                    10,
                    990,
                ),
                sample_claim(
                    4,
                    81,
                    EntityBeliefAspect::Inventory(CommodityKind::Water),
                    ClaimValue::Quantity(Quantity(2)),
                    PerceptionSource::DirectObservation,
                    7,
                    970,
                ),
                sample_claim(
                    5,
                    81,
                    EntityBeliefAspect::Inventory(CommodityKind::Apple),
                    ClaimValue::Quantity(Quantity(1)),
                    PerceptionSource::DirectObservation,
                    6,
                    930,
                ),
            ],
        );

        store.enforce_entity_claim_capacity(&profile(8, 3, 100), Tick(10));

        let claims = store.entity_claims.get(&entity(81)).unwrap();
        assert_eq!(claims.len(), 3);
        assert!(claims.iter().any(|claim| {
            claim.aspect == EntityBeliefAspect::ResourceAvailable(CommodityKind::Apple)
        }));
        assert!(claims.iter().any(|claim| {
            claim.aspect == EntityBeliefAspect::ResourceAvailable(CommodityKind::Bread)
        }));
        assert_eq!(
            claims
                .iter()
                .filter(|claim| matches!(claim.aspect, EntityBeliefAspect::Inventory(_)))
                .count(),
            1
        );
    }

    #[test]
    fn enforce_entity_claim_capacity_respects_within_tier_ordering() {
        let mut store = claim_backed_store(
            82,
            vec![
                sample_claim(
                    1,
                    82,
                    EntityBeliefAspect::ResourceAvailable(CommodityKind::Apple),
                    ClaimValue::ResourceSource(Some(ResourceSource {
                        commodity: CommodityKind::Apple,
                        available_quantity: Quantity(1),
                        max_quantity: Quantity(3),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    })),
                    PerceptionSource::DirectObservation,
                    8,
                    910,
                ),
                sample_claim(
                    2,
                    82,
                    EntityBeliefAspect::ResourceAvailable(CommodityKind::Bread),
                    ClaimValue::ResourceSource(Some(ResourceSource {
                        commodity: CommodityKind::Bread,
                        available_quantity: Quantity(2),
                        max_quantity: Quantity(4),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    })),
                    PerceptionSource::DirectObservation,
                    9,
                    980,
                ),
                sample_claim(
                    3,
                    82,
                    EntityBeliefAspect::ResourceAvailable(CommodityKind::Water),
                    ClaimValue::ResourceSource(Some(ResourceSource {
                        commodity: CommodityKind::Water,
                        available_quantity: Quantity(2),
                        max_quantity: Quantity(4),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    })),
                    PerceptionSource::DirectObservation,
                    10,
                    940,
                ),
            ],
        );

        store.enforce_entity_claim_capacity(&profile(8, 2, 100), Tick(10));

        let claims = store.entity_claims.get(&entity(82)).unwrap();
        assert_eq!(claims.len(), 2);
        assert!(claims.iter().any(|claim| claim.claim_id == ClaimId(2)));
        assert!(claims.iter().any(|claim| claim.claim_id == ClaimId(3)));
        assert!(!claims.iter().any(|claim| claim.claim_id == ClaimId(1)));
    }

    #[test]
    fn enforce_entity_claim_capacity_protects_workstation_present() {
        let mut store = claim_backed_store(
            83,
            vec![
                sample_claim(
                    1,
                    83,
                    EntityBeliefAspect::WorkstationPresent,
                    ClaimValue::WorkstationTag(Some(WorkstationTag::Forge)),
                    PerceptionSource::DirectObservation,
                    8,
                    900,
                ),
                sample_claim(
                    2,
                    83,
                    EntityBeliefAspect::Inventory(CommodityKind::Waste),
                    ClaimValue::Quantity(Quantity(4)),
                    PerceptionSource::DirectObservation,
                    10,
                    990,
                ),
                sample_claim(
                    3,
                    83,
                    EntityBeliefAspect::Inventory(CommodityKind::Water),
                    ClaimValue::Quantity(Quantity(2)),
                    PerceptionSource::DirectObservation,
                    9,
                    950,
                ),
            ],
        );

        store.enforce_entity_claim_capacity(&profile(8, 2, 100), Tick(10));

        let claims = store.entity_claims.get(&entity(83)).unwrap();
        assert_eq!(claims.len(), 2);
        assert!(
            claims
                .iter()
                .any(|claim| claim.aspect == EntityBeliefAspect::WorkstationPresent)
        );
        assert_eq!(
            claims
                .iter()
                .filter(|claim| matches!(claim.aspect, EntityBeliefAspect::Inventory(_)))
                .count(),
            1
        );
    }

    #[test]
    fn enforce_entity_claim_capacity_removes_summary_when_last_claim_is_evicted() {
        let mut store = claim_backed_store(
            62,
            vec![sample_claim(
                1,
                62,
                EntityBeliefAspect::Alive,
                ClaimValue::Bool(true),
                PerceptionSource::DirectObservation,
                1,
                950,
            )],
        );

        store.enforce_entity_claim_capacity(&profile(8, 3, 2), Tick(10));

        assert!(!store.entity_claims.contains_key(&entity(62)));
        assert!(!store.known_entities.contains_key(&entity(62)));
    }

    #[test]
    fn infrastructure_retention_claims_survive_longer() {
        let mut store = claim_backed_store(
            606,
            vec![
                sample_claim(
                    1,
                    606,
                    EntityBeliefAspect::ResourceAvailable(CommodityKind::Apple),
                    ClaimValue::ResourceSource(Some(ResourceSource {
                        commodity: CommodityKind::Apple,
                        available_quantity: Quantity(2),
                        max_quantity: Quantity(5),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    })),
                    PerceptionSource::DirectObservation,
                    0,
                    950,
                ),
                sample_claim(
                    2,
                    606,
                    EntityBeliefAspect::Inventory(CommodityKind::Apple),
                    ClaimValue::Quantity(Quantity(2)),
                    PerceptionSource::DirectObservation,
                    0,
                    950,
                ),
            ],
        );

        let mut perception = profile(10, 10, 48);
        perception.infrastructure_retention_ticks = 480;

        store.enforce_entity_claim_capacity(&perception, Tick(100));

        let claims = store.entity_claims.get(&entity(606)).unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(
            claims[0].aspect,
            EntityBeliefAspect::ResourceAvailable(CommodityKind::Apple)
        );
    }

    #[test]
    fn infrastructure_retention_social_observations_unaffected() {
        let mut store = AgentBeliefStore::new();
        store.record_social_observation(sample_social_observation(0));
        store.record_social_observation(sample_social_observation(47));

        let mut perception = profile(10, 10, 48);
        perception.infrastructure_retention_ticks = 480;

        store.enforce_capacity(&perception, Tick(100));

        assert_eq!(store.social_observations, Vec::<SocialObservation>::new());
    }

    #[test]
    fn enforce_capacity_uses_entity_memory_capacity_not_claim_depth() {
        let mut store = AgentBeliefStore::new();
        for (slot, tick) in [(70_u32, 3_u64), (71_u32, 5_u64)] {
            store.record_entity_claim(sample_claim(
                u64::from(slot),
                slot,
                EntityBeliefAspect::Location,
                ClaimValue::Place(Some(entity(100 + slot))),
                PerceptionSource::DirectObservation,
                tick,
                950,
            ));
            store.record_entity_claim(sample_claim(
                u64::from(slot + 100),
                slot,
                EntityBeliefAspect::Alive,
                ClaimValue::Bool(true),
                PerceptionSource::DirectObservation,
                tick,
                950,
            ));
        }

        store.enforce_capacity(&profile(1, 8, 100), Tick(10));

        assert!(store.known_entities.contains_key(&entity(71)));
        assert!(!store.known_entities.contains_key(&entity(70)));
        assert!(store.entity_claims.contains_key(&entity(71)));
        assert!(!store.entity_claims.contains_key(&entity(70)));
    }

    #[test]
    fn enforce_entity_claim_capacity_uses_claim_depth_not_entity_memory_capacity() {
        let mut store = claim_backed_store(
            72,
            vec![
                sample_claim(
                    1,
                    72,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(10))),
                    PerceptionSource::DirectObservation,
                    7,
                    900,
                ),
                sample_claim(
                    2,
                    72,
                    EntityBeliefAspect::Alive,
                    ClaimValue::Bool(true),
                    PerceptionSource::DirectObservation,
                    8,
                    950,
                ),
                sample_claim(
                    3,
                    72,
                    EntityBeliefAspect::Activity,
                    ClaimValue::Activity(Some(BelievedActivity {
                        action_domain: ActionDomain::Trade,
                        target: Some(entity(11)),
                        observed_tick: Tick(9),
                    })),
                    PerceptionSource::DirectObservation,
                    9,
                    950,
                ),
            ],
        );

        store.enforce_entity_claim_capacity(&profile(1, 2, 100), Tick(10));

        let claims = store.entity_claims.get(&entity(72)).unwrap();
        assert_eq!(claims.len(), 2);
        assert!(
            claims
                .iter()
                .any(|claim| claim.aspect == EntityBeliefAspect::Alive)
        );
        assert!(
            claims
                .iter()
                .any(|claim| claim.aspect == EntityBeliefAspect::Activity)
        );
        assert_eq!(
            store
                .known_entities
                .get(&entity(72))
                .unwrap()
                .believed_activity,
            Some(BelievedActivity {
                action_domain: ActionDomain::Trade,
                target: Some(entity(11)),
                observed_tick: Tick(9),
            })
        );
    }

    */

    #[test]
    fn test_prune_decayed_beliefs_removes_below_threshold() {
        let mut store = AgentBeliefStore::new();

        let mut stale = sample_state(0, 1);
        stale.believed_kind = Some(EntityKind::ItemLot);
        let mut fresh = sample_state(95, 2);
        fresh.believed_kind = Some(EntityKind::ItemLot);

        store.update_entity(entity(1), stale);
        store.update_entity(entity(2), fresh);

        store.prune_decayed_beliefs(
            &profile(100, 50, 5),
            Tick(101),
            &HomeostaticNeeds::new_sated(),
        );

        assert!(!store.known_entities.contains_key(&entity(1)));
        assert!(store.known_entities.contains_key(&entity(2)));
    }

    #[test]
    fn test_social_observation_activation_pruning() {
        let mut store = AgentBeliefStore::new();
        store.record_social_observation(sample_social_observation(0));
        store.record_social_observation(sample_social_observation(100));

        store.prune_decayed_beliefs(
            &profile(100, 50, 5),
            Tick(101),
            &HomeostaticNeeds::new_sated(),
        );

        assert_eq!(
            store.social_observations,
            vec![sample_social_observation(100)]
        );
    }

    #[test]
    fn test_claim_confidence_threshold_prunes_stale_claims() {
        let mut store = claim_backed_store(
            61,
            vec![
                sample_claim(
                    1,
                    61,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(10))),
                    PerceptionSource::DirectObservation,
                    0,
                    950,
                ),
                sample_claim(
                    2,
                    61,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(11))),
                    PerceptionSource::DirectObservation,
                    295,
                    950,
                ),
                sample_claim(
                    3,
                    61,
                    EntityBeliefAspect::Alive,
                    ClaimValue::Bool(true),
                    PerceptionSource::DirectObservation,
                    295,
                    950,
                ),
            ],
        );

        store.prune_decayed_beliefs(
            &profile(100, 50, 5),
            Tick(300),
            &HomeostaticNeeds::new_sated(),
        );

        let claims = store.entity_claims.get(&entity(61)).unwrap();
        assert_eq!(claims.len(), 2);
        assert!(!claims.iter().any(|claim| claim.claim_id == ClaimId(1)));
        assert_eq!(
            store
                .known_entities
                .get(&entity(61))
                .unwrap()
                .last_known_place,
            Some(entity(11))
        );
    }

    #[test]
    fn test_prune_decayed_beliefs_removes_orphan_claims() {
        let mut store = claim_backed_store(
            62,
            vec![sample_claim(
                1,
                62,
                EntityBeliefAspect::Alive,
                ClaimValue::Bool(true),
                PerceptionSource::DirectObservation,
                0,
                950,
            )],
        );

        store.prune_decayed_beliefs(
            &profile(100, 50, 5),
            Tick(300),
            &HomeostaticNeeds::new_sated(),
        );

        assert!(!store.entity_claims.contains_key(&entity(62)));
        assert!(!store.known_entities.contains_key(&entity(62)));
    }

    #[test]
    fn test_prune_decayed_beliefs_refreshes_only_changed_claim_vectors() {
        let changed = entity(90);
        let unchanged = entity(91);
        let mut store = AgentBeliefStore::new();
        store.entity_claims.insert(
            changed,
            vec![
                sample_claim(
                    1,
                    90,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(10))),
                    PerceptionSource::DirectObservation,
                    0,
                    950,
                ),
                sample_claim(
                    2,
                    90,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(11))),
                    PerceptionSource::DirectObservation,
                    295,
                    950,
                ),
                sample_claim(
                    3,
                    90,
                    EntityBeliefAspect::Alive,
                    ClaimValue::Bool(true),
                    PerceptionSource::DirectObservation,
                    295,
                    950,
                ),
            ],
        );
        store.entity_claims.insert(
            unchanged,
            vec![
                sample_claim(
                    4,
                    91,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(20))),
                    PerceptionSource::DirectObservation,
                    295,
                    950,
                ),
                sample_claim(
                    5,
                    91,
                    EntityBeliefAspect::Alive,
                    ClaimValue::Bool(true),
                    PerceptionSource::DirectObservation,
                    295,
                    950,
                ),
            ],
        );
        for subject in [changed, unchanged] {
            store.known_entities.insert(
                subject,
                derive_entity_summary(
                    store.entity_claims.get(&subject).unwrap(),
                    Tick(300),
                    &policy(),
                )
                .unwrap(),
            );
        }
        let unchanged_before = store.get_entity(&unchanged).unwrap().clone();

        store.prune_decayed_beliefs(
            &profile(100, 50, 5),
            Tick(300),
            &HomeostaticNeeds::new_sated(),
        );

        assert_eq!(store.entity_claims.get(&changed).unwrap().len(), 2);
        assert_eq!(
            store.get_entity(&changed).unwrap().last_known_place,
            Some(entity(11))
        );
        assert_eq!(store.entity_claims.get(&unchanged).unwrap().len(), 2);
        assert_eq!(store.get_entity(&unchanged).unwrap(), &unchanged_before);
        assert_eq!(
            store.get_entity(&unchanged),
            derive_entity_summary(
                store.entity_claims.get(&unchanged).unwrap(),
                Tick(300),
                &policy(),
            )
            .as_ref()
        );
    }

    #[test]
    fn test_prune_decayed_beliefs_zero_threshold_refreshes_saturated_ties() {
        let subject = entity(92);
        let mut store = AgentBeliefStore::new();
        store.entity_claims.insert(
            subject,
            vec![
                sample_claim(
                    1,
                    92,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(30))),
                    PerceptionSource::DirectObservation,
                    1,
                    900,
                ),
                sample_claim(
                    2,
                    92,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(31))),
                    PerceptionSource::Report {
                        from: entity(3),
                        chain_len: 1,
                    },
                    9,
                    850,
                ),
                sample_claim(
                    3,
                    92,
                    EntityBeliefAspect::Alive,
                    ClaimValue::Bool(true),
                    PerceptionSource::DirectObservation,
                    9,
                    950,
                ),
            ],
        );
        store.known_entities.insert(
            subject,
            derive_entity_summary(
                store.entity_claims.get(&subject).unwrap(),
                Tick(10),
                &policy(),
            )
            .unwrap(),
        );

        store.prune_decayed_beliefs(
            &profile(0, 0, 5),
            Tick(1_000),
            &HomeostaticNeeds::new_sated(),
        );

        assert_eq!(store.entity_claims.get(&subject).unwrap().len(), 3);
        assert_eq!(
            store.get_entity(&subject),
            derive_entity_summary(
                store.entity_claims.get(&subject).unwrap(),
                Tick(1_000),
                &policy(),
            )
            .as_ref()
        );
        assert_eq!(
            store.get_entity(&subject).unwrap().last_known_place,
            Some(entity(31))
        );
    }

    #[test]
    fn test_prune_preserves_high_activation_entities() {
        let mut store = AgentBeliefStore::new();
        let mut reinforced = sample_state(45, 1);
        reinforced.believed_kind = Some(EntityKind::Facility);
        reinforced.presentation_ticks = [
            Tick(5),
            Tick(15),
            Tick(25),
            Tick(35),
            Tick(45),
            Tick(0),
            Tick(0),
            Tick(0),
        ];
        reinforced.presentation_tick_count = 5;
        store.update_entity(entity(70), reinforced);

        store.prune_decayed_beliefs(
            &profile(100, 50, 5),
            Tick(50),
            &HomeostaticNeeds::new_sated(),
        );

        assert!(store.known_entities.contains_key(&entity(70)));
    }

    #[test]
    fn test_prune_salience_boost_preserves_items() {
        let mut store = AgentBeliefStore::new();

        let mut item = sample_state(0, 1);
        item.believed_kind = Some(EntityKind::ItemLot);
        let mut facility = sample_state(0, 1);
        facility.believed_kind = Some(EntityKind::Facility);

        store.update_entity(entity(80), item);
        store.update_entity(entity(81), facility);

        let mut urgent_needs = HomeostaticNeeds::new_sated();
        urgent_needs.hunger = Permille::new(1000).unwrap();

        store.prune_decayed_beliefs(&profile(100, 50, 5), Tick(400), &urgent_needs);

        assert!(store.known_entities.contains_key(&entity(80)));
        assert!(!store.known_entities.contains_key(&entity(81)));
    }

    #[test]
    fn test_prune_salience_boost_preserves_observed_wash_basin_infrastructure() {
        let mut store = AgentBeliefStore::new();

        let mut basin = sample_state(0, 1);
        basin.believed_kind = Some(EntityKind::Facility);
        basin.workstation_tag = Some(WorkstationTag::WashBasin);
        basin.wash_basin_state = Some(WashBasinState::default());
        let mut other_facility = sample_state(0, 1);
        other_facility.believed_kind = Some(EntityKind::Facility);

        store.update_entity(entity(82), basin);
        store.update_entity(entity(83), other_facility);

        store.prune_decayed_beliefs(
            &profile(100, 50, 5),
            Tick(400),
            &needs_with_pressure(HomeostaticNeedId::Dirtiness),
        );

        assert!(store.known_entities.contains_key(&entity(82)));
        assert!(!store.known_entities.contains_key(&entity(83)));
    }

    #[test]
    fn test_prune_salience_boost_preserves_claim_backed_wash_basin_infrastructure() {
        let mut store = AgentBeliefStore::new();

        let mut basin = sample_state(0, 1);
        basin.believed_kind = Some(EntityKind::Facility);
        basin.workstation_tag = Some(WorkstationTag::WashBasin);
        basin.wash_basin_state = Some(WashBasinState::default());

        store.record_entity_snapshot_claims(
            entity(84),
            &basin,
            None,
            Tick(0),
            Some(Tick(0)),
            5,
            &BeliefConfidencePolicy::default(),
        );

        store.prune_decayed_beliefs(
            &profile(100, 50, 5),
            Tick(400),
            &needs_with_pressure(HomeostaticNeedId::Dirtiness),
        );

        assert!(store.known_entities.contains_key(&entity(84)));
        assert!(
            store
                .get_entity_claims(&entity(84))
                .is_some_and(|claims| !claims.is_empty())
        );
    }

    #[test]
    fn test_prune_salience_boost_preserves_claim_backed_resource_infrastructure() {
        let mut store = AgentBeliefStore::new();

        let mut orchard = sample_state(0, 1);
        orchard.believed_kind = Some(EntityKind::Facility);
        orchard.workstation_tag = Some(WorkstationTag::OrchardRow);
        orchard.resource_source = Some(resource_source(CommodityKind::Apple));

        store.record_entity_snapshot_claims(
            entity(85),
            &orchard,
            None,
            Tick(0),
            Some(Tick(0)),
            5,
            &BeliefConfidencePolicy::default(),
        );

        store.prune_decayed_beliefs(
            &profile(100, 50, 5),
            Tick(400),
            &needs_with_pressure(HomeostaticNeedId::Hunger),
        );

        assert!(store.known_entities.contains_key(&entity(85)));
        assert!(
            store
                .get_entity_claims(&entity(85))
                .is_some_and(|claims| !claims.is_empty())
        );
    }

    #[test]
    fn state_salience_boost_returns_boost_for_each_pressuring_need_with_opportunity_aspect() {
        let urgency_threshold = Permille::new(500).unwrap();
        let boost = Permille::new(500).unwrap();

        let mut basin = sample_state(0, 1);
        basin.believed_kind = Some(EntityKind::Facility);
        basin.workstation_tag = Some(WorkstationTag::WashBasin);
        basin.wash_basin_state = Some(WashBasinState::default());
        assert_eq!(
            state_salience_boost(
                &needs_with_pressure(HomeostaticNeedId::Dirtiness),
                &basin,
                urgency_threshold,
                boost,
            ),
            boost.value()
        );

        let mut orchard = sample_state(0, 1);
        orchard.believed_kind = Some(EntityKind::Facility);
        orchard.workstation_tag = Some(WorkstationTag::OrchardRow);
        orchard.resource_source = Some(resource_source(CommodityKind::Apple));
        assert_eq!(
            state_salience_boost(
                &needs_with_pressure(HomeostaticNeedId::Hunger),
                &orchard,
                urgency_threshold,
                boost,
            ),
            boost.value()
        );

        let mut well = sample_state(0, 1);
        well.believed_kind = Some(EntityKind::Facility);
        well.workstation_tag = Some(WorkstationTag::Well);
        well.resource_source = Some(resource_source(CommodityKind::Water));
        assert_eq!(
            state_salience_boost(
                &needs_with_pressure(HomeostaticNeedId::Thirst),
                &well,
                urgency_threshold,
                boost,
            ),
            boost.value()
        );
    }

    #[test]
    fn state_salience_boost_does_not_boost_unrelated_facility_under_pressure() {
        let mut facility = sample_state(0, 1);
        facility.believed_kind = Some(EntityKind::Facility);
        facility.workstation_tag = Some(WorkstationTag::Mill);

        assert_eq!(
            state_salience_boost(
                &needs_with_pressure(HomeostaticNeedId::Hunger),
                &facility,
                Permille::new(500).unwrap(),
                Permille::new(500).unwrap(),
            ),
            0
        );
    }

    #[test]
    fn state_salience_boost_does_not_boost_resource_that_does_not_relieve_pressuring_need() {
        let mut grain_source = sample_state(0, 1);
        grain_source.believed_kind = Some(EntityKind::Facility);
        grain_source.workstation_tag = Some(WorkstationTag::FieldPlot);
        grain_source.resource_source = Some(resource_source(CommodityKind::Grain));

        assert_eq!(
            state_salience_boost(
                &needs_with_pressure(HomeostaticNeedId::Thirst),
                &grain_source,
                Permille::new(500).unwrap(),
                Permille::new(500).unwrap(),
            ),
            0
        );
    }

    #[test]
    fn state_salience_boost_does_not_boost_indirect_observation_claim_even_with_aspect() {
        let mut basin = sample_state(0, 1);
        basin.believed_kind = Some(EntityKind::Facility);
        basin.workstation_tag = Some(WorkstationTag::WashBasin);
        basin.wash_basin_state = Some(WashBasinState::default());
        basin.source = PerceptionSource::Report {
            from: entity(200),
            chain_len: 1,
        };

        assert_eq!(
            state_salience_boost(
                &needs_with_pressure(HomeostaticNeedId::Dirtiness),
                &basin,
                Permille::new(500).unwrap(),
                Permille::new(500).unwrap(),
            ),
            0
        );
    }

    #[test]
    fn test_refresh_entity_summary_from_claims_preserves_presentation_history() {
        let mut store = claim_backed_store(
            81,
            vec![
                sample_claim(
                    1,
                    81,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(entity(10))),
                    PerceptionSource::DirectObservation,
                    9,
                    950,
                ),
                sample_claim(
                    2,
                    81,
                    EntityBeliefAspect::Alive,
                    ClaimValue::Bool(true),
                    PerceptionSource::DirectObservation,
                    9,
                    950,
                ),
            ],
        );
        let mut prior = sample_state(9, 1);
        prior.presentation_ticks = [
            Tick(1),
            Tick(5),
            Tick(9),
            Tick(0),
            Tick(0),
            Tick(0),
            Tick(0),
            Tick(0),
        ];
        prior.presentation_tick_count = 3;
        store.update_entity(entity(81), prior);

        store.prune_decayed_beliefs(
            &profile(100, 50, 5),
            Tick(10),
            &HomeostaticNeeds::new_sated(),
        );

        let summary = store.known_entities.get(&entity(81)).unwrap();
        assert_eq!(summary.presentation_tick_count, 3);
        assert_eq!(
            &summary.presentation_ticks[..3],
            &[Tick(1), Tick(5), Tick(9)]
        );
    }

    #[test]
    fn record_institutional_belief_enforces_capacity_deterministically() {
        let mut store = AgentBeliefStore::new();
        let mut profile = profile(12, 12, 100);
        profile.institutional_memory_capacity = 2;

        store.record_institutional_belief(
            InstitutionalBeliefKey::OfficeHolderOf { office: entity(70) },
            sample_institutional_belief(5),
            &profile,
        );
        store.record_institutional_belief(
            InstitutionalBeliefKey::FactionMembersOf {
                faction: entity(71),
            },
            sample_institutional_belief(4),
            &profile,
        );
        store.record_institutional_belief(
            InstitutionalBeliefKey::SupportFor {
                supporter: entity(72),
                office: entity(73),
            },
            sample_institutional_belief(6),
            &profile,
        );

        assert!(!store.institutional_beliefs.contains_key(
            &InstitutionalBeliefKey::FactionMembersOf {
                faction: entity(71)
            }
        ));
        assert_eq!(store.total_institutional_beliefs(), 2);
        assert!(
            store
                .institutional_beliefs
                .contains_key(&InstitutionalBeliefKey::OfficeHolderOf { office: entity(70) })
        );
        assert!(
            store
                .institutional_beliefs
                .contains_key(&InstitutionalBeliefKey::SupportFor {
                    supporter: entity(72),
                    office: entity(73),
                })
        );
    }

    #[test]
    fn record_institutional_belief_breaks_ties_by_key_then_position() {
        let mut store = AgentBeliefStore::new();
        let mut profile = profile(12, 12, 100);
        profile.institutional_memory_capacity = 2;
        let first_key = InstitutionalBeliefKey::FactionMembersOf {
            faction: entity(80),
        };
        let second_key = InstitutionalBeliefKey::SupportFor {
            supporter: entity(81),
            office: entity(82),
        };

        store.record_institutional_belief(first_key, sample_institutional_belief(5), &profile);
        store.record_institutional_belief(second_key, sample_institutional_belief(5), &profile);
        store.record_institutional_belief(
            InstitutionalBeliefKey::OfficeHolderOf { office: entity(83) },
            sample_institutional_belief(6),
            &profile,
        );

        assert!(!store.institutional_beliefs.contains_key(&first_key));
        assert!(store.institutional_beliefs.contains_key(&second_key));
    }

    #[test]
    fn record_institutional_belief_clears_all_when_capacity_is_zero() {
        let mut store = AgentBeliefStore::new();
        let mut profile = profile(12, 12, 100);
        profile.institutional_memory_capacity = 0;

        store.record_institutional_belief(
            InstitutionalBeliefKey::OfficeHolderOf { office: entity(90) },
            sample_institutional_belief(7),
            &profile,
        );

        assert!(store.institutional_beliefs.is_empty());
    }

    #[test]
    fn replace_institutional_belief_overwrites_existing_key_without_conflict() {
        let mut store = AgentBeliefStore::new();
        let mut profile = profile(12, 12, 100);
        profile.institutional_memory_capacity = 4;
        let office = entity(91);
        let supporter = entity(92);
        let key = InstitutionalBeliefKey::SupportFor { supporter, office };

        store.record_institutional_belief(
            key,
            BelievedInstitutionalClaim {
                claim: InstitutionalClaim::SupportDeclaration {
                    office,
                    supporter,
                    candidate: Some(entity(93)),
                    effective_tick: Tick(5),
                },
                source: InstitutionalKnowledgeSource::SelfDeclaration,
                learned_tick: Tick(5),
                learned_at: Some(entity(7)),
            },
            &profile,
        );
        store.replace_institutional_belief(
            key,
            BelievedInstitutionalClaim {
                claim: InstitutionalClaim::SupportDeclaration {
                    office,
                    supporter,
                    candidate: Some(entity(94)),
                    effective_tick: Tick(6),
                },
                source: InstitutionalKnowledgeSource::SelfDeclaration,
                learned_tick: Tick(6),
                learned_at: Some(entity(7)),
            },
            &profile,
        );

        assert_eq!(
            store.believed_support_declaration(office, supporter),
            InstitutionalBeliefRead::Certain(Some(entity(94)))
        );
        assert_eq!(
            store
                .institutional_beliefs
                .get(&key)
                .expect("support belief should remain present")
                .len(),
            1
        );
    }

    #[test]
    fn believed_office_holder_returns_unknown_when_absent() {
        let store = AgentBeliefStore::new();

        assert_eq!(
            store.believed_office_holder(entity(70)),
            InstitutionalBeliefRead::Unknown
        );
    }

    #[test]
    fn believed_office_holder_collapses_agreeing_claims_and_preserves_vacancy() {
        let mut store = AgentBeliefStore::new();
        let office = entity(71);
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::OfficeHolderOf { office },
            vec![
                office_holder_belief(71, None, InstitutionalKnowledgeSource::WitnessedEvent, 4),
                office_holder_belief(
                    71,
                    None,
                    InstitutionalKnowledgeSource::Report {
                        from: entity(72),
                        chain_len: 1,
                    },
                    7,
                ),
            ],
        );

        assert_eq!(
            store.believed_office_holder(office),
            InstitutionalBeliefRead::Certain(None)
        );
    }

    #[test]
    fn believed_office_holder_returns_conflicted_for_distinct_holders() {
        let mut store = AgentBeliefStore::new();
        let office = entity(73);
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::OfficeHolderOf { office },
            vec![
                office_holder_belief(
                    73,
                    Some(74),
                    InstitutionalKnowledgeSource::WitnessedEvent,
                    5,
                ),
                office_holder_belief(
                    73,
                    Some(75),
                    InstitutionalKnowledgeSource::SelfDeclaration,
                    8,
                ),
            ],
        );

        assert_eq!(
            store.believed_office_holder(office),
            InstitutionalBeliefRead::Conflicted(vec![Some(entity(74)), Some(entity(75))])
        );
    }

    #[test]
    fn believed_force_controller_returns_unknown_when_absent() {
        let store = AgentBeliefStore::new();

        assert_eq!(
            store.believed_force_controller(entity(76)),
            InstitutionalBeliefRead::Unknown
        );
    }

    #[test]
    fn believed_force_controller_collapses_agreeing_claims() {
        let mut store = AgentBeliefStore::new();
        let office = entity(77);
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::ForceControllerOf { office },
            vec![
                force_control_belief(77, Some(78), false, 4),
                force_control_belief(77, Some(78), false, 6),
            ],
        );

        assert_eq!(
            store.believed_force_controller(office),
            InstitutionalBeliefRead::Certain((Some(entity(78)), false))
        );
    }

    #[test]
    fn believed_force_controller_returns_conflicted_for_distinct_reads() {
        let mut store = AgentBeliefStore::new();
        let office = entity(79);
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::ForceControllerOf { office },
            vec![
                force_control_belief(79, Some(80), false, 3),
                force_control_belief(79, None, true, 5),
            ],
        );

        assert_eq!(
            store.believed_force_controller(office),
            InstitutionalBeliefRead::Conflicted(vec![(None, true), (Some(entity(80)), false)])
        );
    }

    #[test]
    fn believed_membership_filters_to_the_queried_member() {
        let mut store = AgentBeliefStore::new();
        let faction = entity(80);
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::FactionMembersOf { faction },
            vec![
                membership_belief(80, 81, true, 3),
                membership_belief(80, 82, false, 4),
                membership_belief(80, 81, true, 6),
            ],
        );

        assert_eq!(
            store.believed_membership(faction, entity(81)),
            InstitutionalBeliefRead::Certain(true)
        );
        assert_eq!(
            store.believed_membership(faction, entity(82)),
            InstitutionalBeliefRead::Certain(false)
        );
        assert_eq!(
            store.believed_membership(faction, entity(83)),
            InstitutionalBeliefRead::Unknown
        );
    }

    #[test]
    fn believed_membership_returns_conflicted_for_same_member_with_distinct_values() {
        let mut store = AgentBeliefStore::new();
        let faction = entity(84);
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::FactionMembersOf { faction },
            vec![
                membership_belief(84, 85, true, 2),
                membership_belief(84, 85, false, 5),
            ],
        );

        assert_eq!(
            store.believed_membership(faction, entity(85)),
            InstitutionalBeliefRead::Conflicted(vec![false, true])
        );
    }

    #[test]
    fn believed_faction_rally_point_reads_matching_claims_only() {
        let mut store = AgentBeliefStore::new();
        let faction = entity(86);
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::FactionRallyPointOf { faction },
            vec![
                rally_point_belief(86, Some(87), 2),
                membership_belief(86, 99, true, 3),
                rally_point_belief(86, Some(87), 5),
            ],
        );

        assert_eq!(
            store.believed_faction_rally_point(faction),
            InstitutionalBeliefRead::Certain(Some(entity(87)))
        );
    }

    #[test]
    fn believed_faction_rally_point_returns_conflicted_for_distinct_places() {
        let mut store = AgentBeliefStore::new();
        let faction = entity(88);
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::FactionRallyPointOf { faction },
            vec![
                rally_point_belief(88, Some(89), 2),
                rally_point_belief(88, None, 5),
            ],
        );

        assert_eq!(
            store.believed_faction_rally_point(faction),
            InstitutionalBeliefRead::Conflicted(vec![None, Some(entity(89))])
        );
    }

    #[test]
    fn believed_support_declaration_ignores_malformed_claims_under_matching_key() {
        let mut store = AgentBeliefStore::new();
        let office = entity(90);
        let supporter = entity(91);
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::SupportFor { supporter, office },
            vec![
                office_holder_belief(
                    90,
                    Some(99),
                    InstitutionalKnowledgeSource::WitnessedEvent,
                    2,
                ),
                support_belief(90, 91, Some(92), 6),
            ],
        );

        assert_eq!(
            store.believed_support_declaration(office, supporter),
            InstitutionalBeliefRead::Certain(Some(entity(92)))
        );
    }

    #[test]
    fn believed_support_declarations_for_office_groups_reads_by_supporter() {
        let mut store = AgentBeliefStore::new();
        let office = entity(100);
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::SupportFor {
                supporter: entity(101),
                office,
            },
            vec![support_belief(100, 101, Some(103), 3)],
        );
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::SupportFor {
                supporter: entity(102),
                office,
            },
            vec![
                support_belief(100, 102, Some(104), 4),
                support_belief(100, 102, None, 7),
            ],
        );
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::SupportFor {
                supporter: entity(105),
                office: entity(106),
            },
            vec![support_belief(106, 105, Some(107), 5)],
        );

        assert_eq!(
            store.believed_support_declarations_for_office(office),
            vec![
                (
                    entity(101),
                    InstitutionalBeliefRead::Certain(Some(entity(103))),
                ),
                (
                    entity(102),
                    InstitutionalBeliefRead::Conflicted(vec![None, Some(entity(104))]),
                ),
            ]
        );
    }

    #[test]
    fn believed_entity_state_roundtrips_through_bincode() {
        let state = sample_state(11, 7);

        let bytes = bincode::serialize(&state).unwrap();
        let roundtrip: BelievedEntityState = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, state);
    }

    #[test]
    fn believed_activity_constructs_and_compares() {
        let activity = BelievedActivity {
            action_domain: ActionDomain::Production,
            target: Some(entity(44)),
            observed_tick: Tick(13),
        };

        assert_eq!(
            activity,
            BelievedActivity {
                action_domain: ActionDomain::Production,
                target: Some(entity(44)),
                observed_tick: Tick(13),
            }
        );
        assert_ne!(
            activity,
            BelievedActivity {
                action_domain: ActionDomain::Trade,
                target: None,
                observed_tick: Tick(13),
            }
        );
    }

    #[test]
    fn believed_entity_state_equality_includes_believed_activity() {
        let mut with_activity = sample_state(11, 7);
        with_activity.believed_activity = Some(BelievedActivity {
            action_domain: ActionDomain::Trade,
            target: Some(entity(21)),
            observed_tick: Tick(11),
        });

        let without_activity = sample_state(11, 7);

        assert_ne!(with_activity, without_activity);
        assert_eq!(without_activity.believed_activity, None);
    }

    #[test]
    fn observed_entity_snapshot_roundtrips_through_bincode() {
        let snapshot = ObservedEntitySnapshot {
            believed_kind: Some(EntityKind::Agent),
            last_known_place: Some(entity(10)),
            last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(3))]),
            workstation_tag: None,
            resource_source: None,
            wash_basin_state: None,
            alive: true,
            wounds: vec![sample_wound(1, 4)],
            courage: None,
            artifact_state: None,
            contention_state: Some(BelievedContentionState {
                grant_holder: Some(entity(11)),
                queue_length: 2,
                observed_tick: Tick(4),
            }),
            evidence_state: None,
        };

        let bytes = bincode::serialize(&snapshot).unwrap();
        let roundtrip: ObservedEntitySnapshot = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, snapshot);
    }

    #[test]
    fn perception_source_roundtrips_and_compares() {
        let source = PerceptionSource::Report {
            from: entity(7),
            chain_len: 2,
        };

        let bytes = bincode::serialize(&source).unwrap();
        let roundtrip: PerceptionSource = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, source);
        assert_ne!(source, PerceptionSource::Inference);
    }

    #[test]
    fn social_observation_kind_roundtrips_and_compares() {
        let kind = SocialObservationKind::WitnessedTelling;

        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: SocialObservationKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, kind);
        assert_ne!(kind, SocialObservationKind::WitnessedConflict);
        assert_ne!(kind, SocialObservationKind::WitnessedCooperation);
    }

    #[test]
    fn witnessed_absence_roundtrips_and_differs_from_others() {
        let kind = SocialObservationKind::WitnessedAbsence;

        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: SocialObservationKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, kind);
        assert_ne!(kind, SocialObservationKind::WitnessedTelling);
        assert_ne!(kind, SocialObservationKind::CoPresence);
    }

    #[test]
    fn suspected_theft_roundtrips_and_differs_from_witnessed_absence() {
        let kind = SocialObservationKind::SuspectedTheft;

        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: SocialObservationKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, kind);
        assert_ne!(kind, SocialObservationKind::WitnessedAbsence);
        assert_ne!(kind, SocialObservationKind::WitnessedTelling);
    }

    #[test]
    fn social_observation_detail_roundtrips_and_derives_kind() {
        let detail = SocialObservationDetail::SuspectedTheft {
            theft: TheftFacts {
                missing_entity: entity(21),
                expected_place: entity(22),
                commodity: CommodityKind::Bread,
                quantity: Quantity(2),
            },
            suspect: Some(entity(23)),
        };

        let bytes = bincode::serialize(&detail).unwrap();
        let roundtrip: SocialObservationDetail = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, detail);
        assert_eq!(detail.kind(), SocialObservationKind::SuspectedTheft);
        assert_ne!(detail.kind(), SocialObservationKind::WitnessedAbsence);
    }

    #[test]
    fn mismatch_kind_variants_construct_and_sort_stably() {
        let mut variants = [
            MismatchKind::PlaceChanged {
                believed_place: entity(4),
                observed_place: entity(5),
            },
            MismatchKind::EntityMissing,
            MismatchKind::InventoryDiscrepancy {
                commodity: CommodityKind::Bread,
                believed: Quantity(5),
                observed: Quantity(2),
            },
            MismatchKind::ResourceSourceDiscrepancy {
                commodity: CommodityKind::Apple,
                believed: Quantity(9),
                observed: Quantity(1),
            },
            MismatchKind::AliveStatusChanged,
        ];

        variants.sort_unstable();

        assert_eq!(
            variants,
            [
                MismatchKind::EntityMissing,
                MismatchKind::AliveStatusChanged,
                MismatchKind::InventoryDiscrepancy {
                    commodity: CommodityKind::Bread,
                    believed: Quantity(5),
                    observed: Quantity(2),
                },
                MismatchKind::ResourceSourceDiscrepancy {
                    commodity: CommodityKind::Apple,
                    believed: Quantity(9),
                    observed: Quantity(1),
                },
                MismatchKind::PlaceChanged {
                    believed_place: entity(4),
                    observed_place: entity(5),
                },
            ]
        );
    }

    #[test]
    fn mismatch_kind_roundtrips_through_bincode() {
        let mismatch = MismatchKind::InventoryDiscrepancy {
            commodity: CommodityKind::Water,
            believed: Quantity(7),
            observed: Quantity(3),
        };

        let bytes = bincode::serialize(&mismatch).unwrap();
        let roundtrip: MismatchKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, mismatch);
    }

    #[test]
    fn perception_profile_roundtrips_through_bincode() {
        let profile = profile(12, 7, 34);

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: PerceptionProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn agent_belief_store_roundtrips_through_bincode_with_institutional_beliefs() {
        let mut store = AgentBeliefStore::new();
        store.record_entity_claim(sample_claim(
            1,
            1,
            EntityBeliefAspect::Alive,
            ClaimValue::Bool(true),
            PerceptionSource::DirectObservation,
            7,
            950,
        ));
        store.update_entity(entity(1), sample_state(7, 2));
        store.place_visits.insert(
            entity(49),
            PlaceVisitRecord {
                ticks_present: 12,
                last_arrival_tick: Tick(9),
                visit_count: 3,
            },
        );
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::OfficeHolderOf { office: entity(50) },
            vec![sample_institutional_belief(12)],
        );

        let bytes = bincode::serialize(&store).unwrap();
        let roundtrip: AgentBeliefStore = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, store);
    }

    #[test]
    fn entity_belief_claim_roundtrips_refuted_at_tick_through_bincode() {
        let claim = EntityBeliefClaim {
            refuted_at_tick: Some(Tick(19)),
            ..sample_claim(
                1,
                1,
                EntityBeliefAspect::Alive,
                ClaimValue::Bool(true),
                PerceptionSource::DirectObservation,
                7,
                950,
            )
        };

        let bytes = bincode::serialize(&claim).unwrap();
        let roundtrip: EntityBeliefClaim = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, claim);
    }

    #[test]
    fn belief_confidence_policy_roundtrips_through_bincode() {
        let policy = BeliefConfidencePolicy {
            direct_observation_base: Permille::new(920).unwrap(),
            report_base: Permille::new(730).unwrap(),
            rumor_base: Permille::new(510).unwrap(),
            inference_base: Permille::new(390).unwrap(),
            report_chain_penalty: Permille::new(70).unwrap(),
            rumor_chain_penalty: Permille::new(95).unwrap(),
            staleness_penalty_per_tick: Permille::new(8).unwrap(),
        };

        let bytes = bincode::serialize(&policy).unwrap();
        let roundtrip: BeliefConfidencePolicy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, policy);
    }

    #[test]
    fn tell_profile_defaults_match_e15_spec() {
        assert_eq!(
            TellProfile::default(),
            TellProfile {
                max_tell_candidates: 3,
                max_relay_chain_len: 3,
                conversation_memory_capacity: 12,
                conversation_memory_retention_ticks: 48,
            }
        );
    }

    #[test]
    fn tell_profile_roundtrips_through_bincode() {
        let profile = TellProfile {
            max_tell_candidates: 5,
            max_relay_chain_len: 2,
            conversation_memory_capacity: 9,
            conversation_memory_retention_ticks: 21,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: TellProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn entity_belief_claim_roundtrips_through_bincode() {
        let claim = sample_claim(
            8,
            77,
            EntityBeliefAspect::WashBasinState,
            ClaimValue::WashBasinState(Some(WashBasinState {
                clean_water_units: 9,
                units_per_full_wash: 2,
                dirtiness_level: Permille::new(125).unwrap(),
                ..WashBasinState::default()
            })),
            PerceptionSource::Report {
                from: entity(55),
                chain_len: 1,
            },
            8,
            730,
        );

        let bytes = bincode::serialize(&claim).unwrap();
        let roundtrip: EntityBeliefClaim = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, claim);
    }

    #[test]
    fn shared_belief_snapshot_ignores_observed_tick_and_matches_shareable_content() {
        let older = sample_state(3, 4);
        let mut newer = older.clone();
        newer.push_presentation_tick(Tick(9), 5);
        let snapshot = to_shared_belief_snapshot(&older);

        assert_eq!(snapshot, to_shared_belief_snapshot(&newer));
        assert!(share_equivalent(&newer, &snapshot));
    }

    #[test]
    fn conversation_memory_read_helpers_ignore_expired_entries_before_cleanup() {
        let mut store = AgentBeliefStore::new();
        let profile = tell_profile();
        let fresh_state = sample_state(9, 3);
        let stale_state = sample_state(1, 2);
        let stale_key = tell_memory_key(2, 21);
        let fresh_key = tell_memory_key(3, 22);

        let stale_told = told_memory(2, 21, 1, &stale_state);
        let fresh_told = told_memory(3, 22, 9, &fresh_state);
        let stale_heard = heard_memory(2, 21, 1, &stale_state, HeardBeliefDisposition::Accepted);
        let fresh_heard = heard_memory(
            3,
            22,
            9,
            &fresh_state,
            HeardBeliefDisposition::AlreadyHeldEqualOrNewer,
        );

        store.record_told_belief(stale_told.0, stale_told.1);
        store.record_told_belief(fresh_told.0, fresh_told.1);
        store.record_heard_belief(stale_heard.0, stale_heard.1);
        store.record_heard_belief(fresh_heard.0, fresh_heard.1);

        assert_eq!(
            store.told_belief_memory(&stale_key, Tick(9), &profile),
            None
        );
        assert_eq!(
            store.heard_belief_memory(&stale_key, Tick(9), &profile),
            None
        );
        assert!(store.told_beliefs.contains_key(&stale_key));
        assert!(store.heard_beliefs.contains_key(&stale_key));
        assert_eq!(
            store.recipient_knowledge_status(
                &stale_key,
                &SharedTellState::EntityBelief(to_shared_belief_snapshot(&fresh_state)),
                Tick(9),
                &profile
            ),
            RecipientKnowledgeStatus::SpeakerPreviouslyToldButMemoryExpired
        );

        assert_eq!(
            store
                .told_belief_memory(&fresh_key, Tick(9), &profile)
                .map(|_| fresh_key),
            Some(fresh_key)
        );
        assert_eq!(
            store
                .heard_belief_memory(&fresh_key, Tick(9), &profile)
                .map(|_| fresh_key),
            Some(fresh_key)
        );
    }

    #[test]
    fn enforce_conversation_memory_evicts_oldest_told_and_heard_entries_independently() {
        let mut store = AgentBeliefStore::new();
        let profile = tell_profile();

        let told_a = told_memory(2, 20, 4, &sample_state(4, 1));
        let told_b = told_memory(1, 10, 4, &sample_state(4, 2));
        let told_c = told_memory(3, 30, 6, &sample_state(6, 3));
        let heard_a = heard_memory(
            5,
            50,
            3,
            &sample_state(3, 1),
            HeardBeliefDisposition::Accepted,
        );
        let heard_b = heard_memory(
            4,
            40,
            3,
            &sample_state(3, 2),
            HeardBeliefDisposition::Accepted,
        );
        let heard_c = heard_memory(
            6,
            60,
            7,
            &sample_state(7, 3),
            HeardBeliefDisposition::Accepted,
        );

        store.record_told_belief(told_a.0, told_a.1);
        store.record_told_belief(told_b.0, told_b.1);
        store.record_told_belief(told_c.0, told_c.1);
        store.record_heard_belief(heard_a.0, heard_a.1);
        store.record_heard_belief(heard_b.0, heard_b.1);
        store.record_heard_belief(heard_c.0, heard_c.1);

        store.enforce_conversation_memory(&profile, Tick(8));

        assert_eq!(
            store.told_beliefs.keys().copied().collect::<Vec<_>>(),
            vec![tell_memory_key(2, 20), tell_memory_key(3, 30)]
        );
        assert_eq!(
            store.heard_beliefs.keys().copied().collect::<Vec<_>>(),
            vec![tell_memory_key(5, 50), tell_memory_key(6, 60)]
        );
    }

    #[test]
    fn ask_witness_memory_filters_expired_entries_by_retention_window() {
        let mut store = AgentBeliefStore::new();
        let stale_key = ask_memory_key(7, Some(44), Some(CommodityKind::Bread));
        let fresh_key = ask_memory_key(8, Some(45), None);
        store.record_asked_witness(
            stale_key,
            AskWitnessMemory {
                asked_tick: Tick(3),
            },
        );
        store.record_asked_witness(
            fresh_key,
            AskWitnessMemory {
                asked_tick: Tick(7),
            },
        );

        assert_eq!(store.ask_witness_memory(&stale_key, Tick(9), 4), None);
        assert_eq!(
            store
                .ask_witness_memory(&fresh_key, Tick(9), 4)
                .map(|memory| memory.asked_tick),
            Some(Tick(7))
        );
    }

    #[test]
    fn enforce_ask_witness_memory_prunes_expired_entries() {
        let mut store = AgentBeliefStore::new();
        let stale_key = ask_memory_key(7, Some(44), None);
        let fresh_key = ask_memory_key(8, None, Some(CommodityKind::Apple));
        store.record_asked_witness(
            stale_key,
            AskWitnessMemory {
                asked_tick: Tick(2),
            },
        );
        store.record_asked_witness(
            fresh_key,
            AskWitnessMemory {
                asked_tick: Tick(8),
            },
        );

        store.enforce_ask_witness_memory(Tick(9), 3);

        assert!(!store.asked_witnesses.contains_key(&stale_key));
        assert!(store.asked_witnesses.contains_key(&fresh_key));
    }

    #[test]
    fn recipient_knowledge_status_distinguishes_current_and_stale_tells() {
        let current = sample_state(8, 4);
        let stale = sample_state(8, 9);
        let (_, remembered) = told_memory(7, 44, 6, &current);

        assert_eq!(
            recipient_knowledge_status(
                &SharedTellState::EntityBelief(to_shared_belief_snapshot(&current)),
                Some(&remembered)
            ),
            RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
        );
        assert_eq!(
            recipient_knowledge_status(
                &SharedTellState::EntityBelief(to_shared_belief_snapshot(&stale)),
                Some(&remembered)
            ),
            RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief
        );
        assert_eq!(
            recipient_knowledge_status(
                &SharedTellState::EntityBelief(to_shared_belief_snapshot(&current)),
                None
            ),
            RecipientKnowledgeStatus::UnknownToSpeaker
        );
    }

    #[test]
    fn recipient_knowledge_status_ignores_entity_provenance_only_changes() {
        let current = sample_state(8, 4);
        let mut echoed = current.clone();
        echoed.source = PerceptionSource::Report {
            from: entity(77),
            chain_len: 1,
        };
        let (_, remembered) = told_memory(7, 44, 6, &current);

        assert_eq!(
            recipient_knowledge_status(
                &SharedTellState::EntityBelief(to_shared_belief_snapshot(&echoed)),
                Some(&remembered)
            ),
            RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
        );
    }

    #[test]
    fn recipient_knowledge_status_ignores_social_observation_provenance_only_changes() {
        let current = sample_social_observation(4);
        let mut echoed = current;
        echoed.source = PerceptionSource::Report {
            from: entity(77),
            chain_len: 1,
        };
        let remembered = ToldBeliefMemory {
            shared_state: SharedTellState::SocialObservation(current),
            told_tick: Tick(6),
        };

        assert_eq!(
            recipient_knowledge_status(
                &SharedTellState::SocialObservation(echoed),
                Some(&remembered)
            ),
            RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
        );
    }

    #[test]
    fn recipient_knowledge_status_treats_changed_institutional_topic_as_new_shareable_content() {
        let tell_profile = tell_profile();
        let office = entity(44);
        let key = TellMemoryKey {
            counterparty: entity(7),
            topic: TellTopic::InstitutionalClaim {
                claim: InstitutionalClaim::OfficeHolder {
                    office,
                    holder: None,
                    effective_tick: Tick(6),
                },
            },
        };
        let mut store = AgentBeliefStore::new();
        let memory = ToldBeliefMemory {
            shared_state: SharedTellState::InstitutionalClaim(SharedInstitutionalBelief {
                claim: InstitutionalClaim::OfficeHolder {
                    office,
                    holder: None,
                    effective_tick: Tick(6),
                },
                source: InstitutionalKnowledgeSource::WitnessedEvent,
            }),
            told_tick: Tick(6),
        };
        store.record_told_belief(key, memory);
        store.record_institutional_belief(
            InstitutionalBeliefKey::OfficeHolderOf { office },
            office_holder_belief(
                office.slot,
                Some(11),
                InstitutionalKnowledgeSource::WitnessedEvent,
                8,
            ),
            &profile(8, 8, 100),
        );

        assert_eq!(
            store.recipient_knowledge_status(
                &key,
                &store
                    .shared_tell_state_for_topic(&key.topic, tell_profile.max_relay_chain_len)
                    .unwrap(),
                Tick(8),
                &tell_profile
            ),
            RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief
        );
    }

    #[test]
    fn crime_case_claims_share_a_memory_lane_by_accused_and_violation() {
        let accusation = InstitutionalClaim::Accusation {
            accuser: entity(70),
            accused: entity(80),
            violation_id: crate::ViolationId(9),
            theft: TheftFacts {
                missing_entity: entity(81),
                expected_place: entity(82),
                commodity: CommodityKind::Coin,
                quantity: Quantity(4),
            },
            effective_tick: Tick(5),
        };
        let verdict = InstitutionalClaim::Verdict {
            accused: entity(80),
            violation_id: crate::ViolationId(9),
            punishment: crate::PunishmentKind::Fine {
                commodity: CommodityKind::Coin,
                amount: Quantity(4),
            },
            effective_tick: Tick(7),
        };
        let other_case = InstitutionalClaim::Accusation {
            accuser: entity(70),
            accused: entity(80),
            violation_id: crate::ViolationId(10),
            theft: TheftFacts {
                missing_entity: entity(83),
                expected_place: entity(84),
                commodity: CommodityKind::Coin,
                quantity: Quantity(1),
            },
            effective_tick: Tick(8),
        };

        assert!(institutional_claim_same_memory_lane(accusation, verdict));
        assert!(!institutional_claim_same_memory_lane(
            accusation, other_case
        ));
    }

    #[test]
    fn current_institutional_belief_topics_prefers_newer_verdict_for_same_crime_case() {
        let current = current_institutional_belief_topics(vec![
            accusation_belief(80, 9, 5),
            verdict_belief(80, 9, 7),
        ]);

        assert_eq!(current.len(), 1);
        assert_eq!(current[0], verdict_belief(80, 9, 7));
    }

    #[test]
    fn belief_confidence_orders_sources_by_provenance() {
        let policy = policy();
        let direct = belief_confidence(&PerceptionSource::DirectObservation, 0, &policy);
        let report = belief_confidence(
            &PerceptionSource::Report {
                from: entity(7),
                chain_len: 1,
            },
            0,
            &policy,
        );
        let rumor = belief_confidence(&PerceptionSource::Rumor { chain_len: 1 }, 0, &policy);
        let inference = belief_confidence(&PerceptionSource::Inference, 0, &policy);

        assert!(direct > report);
        assert!(report > rumor);
        assert!(rumor > inference);
        assert_eq!(direct, policy.direct_observation_base);
    }

    #[test]
    fn belief_confidence_penalizes_deeper_report_and_rumor_chains() {
        let policy = policy();
        let report_shallow = belief_confidence(
            &PerceptionSource::Report {
                from: entity(1),
                chain_len: 1,
            },
            0,
            &policy,
        );
        let report_deep = belief_confidence(
            &PerceptionSource::Report {
                from: entity(1),
                chain_len: 3,
            },
            0,
            &policy,
        );
        let rumor_shallow =
            belief_confidence(&PerceptionSource::Rumor { chain_len: 1 }, 0, &policy);
        let rumor_deep = belief_confidence(&PerceptionSource::Rumor { chain_len: 3 }, 0, &policy);

        assert!(report_deep < report_shallow);
        assert!(rumor_deep < rumor_shallow);
    }

    #[test]
    fn belief_confidence_monotonically_decays_with_staleness() {
        let policy = policy();
        let fresh = belief_confidence(&PerceptionSource::DirectObservation, 0, &policy);
        let slightly_stale = belief_confidence(&PerceptionSource::DirectObservation, 5, &policy);
        let stale = belief_confidence(&PerceptionSource::DirectObservation, 10, &policy);

        assert!(slightly_stale < fresh);
        assert!(stale < slightly_stale);
    }

    #[test]
    fn belief_confidence_saturates_at_zero_for_large_staleness() {
        let policy = policy();
        let stale_report = belief_confidence(
            &PerceptionSource::Report {
                from: entity(4),
                chain_len: 5,
            },
            u64::MAX,
            &policy,
        );

        assert_eq!(stale_report, Permille::new(0).unwrap());
    }

    #[test]
    fn belief_confidence_is_deterministic_for_identical_inputs() {
        let policy = policy();
        let source = PerceptionSource::Report {
            from: entity(9),
            chain_len: 2,
        };

        assert_eq!(
            belief_confidence(&source, 7, &policy),
            belief_confidence(&source, 7, &policy)
        );
    }

    #[test]
    fn belief_confidence_uses_custom_policy_values() {
        let custom_policy = BeliefConfidencePolicy {
            direct_observation_base: Permille::new(700).unwrap(),
            report_base: Permille::new(680).unwrap(),
            rumor_base: Permille::new(660).unwrap(),
            inference_base: Permille::new(640).unwrap(),
            report_chain_penalty: Permille::new(15).unwrap(),
            rumor_chain_penalty: Permille::new(20).unwrap(),
            staleness_penalty_per_tick: Permille::new(3).unwrap(),
        };

        let custom_report = belief_confidence(
            &PerceptionSource::Report {
                from: entity(11),
                chain_len: 3,
            },
            4,
            &custom_policy,
        );

        assert_eq!(custom_report, Permille::new(638).unwrap());
    }

    #[test]
    fn perception_profile_default_includes_activation_decay_fields() {
        let profile = PerceptionProfile::default();

        assert_eq!(profile.confidence_policy, BeliefConfidencePolicy::default());
        assert_eq!(
            profile.entity_activation_threshold,
            Permille::new(100).unwrap()
        );
        assert_eq!(
            profile.claim_confidence_threshold,
            Permille::new(50).unwrap()
        );
        assert_eq!(profile.observation_buffer_capacity, 5);
        assert_eq!(profile.observation_budget, 24);
        assert_eq!(
            profile.salience_policy,
            SaliencePolicy::PriorityWithNeedBoost
        );
        assert_eq!(profile.omission_log_capacity, 16);
        assert_eq!(profile.need_salience_boost, Permille::new(500).unwrap());
        assert_eq!(
            profile.need_salience_urgency_threshold,
            Permille::new(500).unwrap()
        );
        assert_eq!(profile.institutional_memory_capacity, 20);
        assert_eq!(
            profile.consultation_speed_factor,
            Permille::new(500).unwrap()
        );
        assert_eq!(profile.contradiction_tolerance, Permille::new(300).unwrap());
    }

    #[test]
    fn perception_profile_serde_defaults_observation_budget_when_omitted() {
        let serialized = ron::to_string(&PerceptionProfile::default()).expect("serialize");
        let omitted = serialized
            .replace("observation_budget:24,", "")
            .replace("observation_budget: 24,", "")
            .replace("salience_policy:PriorityWithNeedBoost,", "")
            .replace("salience_policy: PriorityWithNeedBoost,", "")
            .replace("omission_log_capacity:16,", "")
            .replace("omission_log_capacity: 16,", "");
        let profile: PerceptionProfile = ron::from_str(&omitted)
            .expect("deserialize perception profile without defaulted fields");

        assert_eq!(profile.observation_budget, 24);
        assert_eq!(
            profile.salience_policy,
            SaliencePolicy::PriorityWithNeedBoost
        );
        assert_eq!(profile.omission_log_capacity, 16);
    }

    #[test]
    fn perception_profile_serde_accepts_explicit_observation_budget() {
        let serialized = ron::to_string(&PerceptionProfile::default()).expect("serialize");
        let explicit = serialized
            .replace("observation_budget:24", "observation_budget:11")
            .replace("observation_budget: 24", "observation_budget: 11");
        let profile: PerceptionProfile = ron::from_str(&explicit)
            .expect("deserialize perception profile with explicit observation_budget");

        assert_eq!(profile.observation_budget, 11);
    }

    #[test]
    fn observation_omission_types_roundtrip_and_defaults_hold() {
        let omission = ObservationOmission {
            omitted_entity: entity(17),
            reason: OmissionReason::OverBudget {
                budget: 12,
                candidates_seen: 30,
            },
            observed_tick: Tick(42),
        };
        let bytes = bincode::serialize(&omission).expect("serialize omission");
        let restored: ObservationOmission = bincode::deserialize(&bytes).expect("deserialize");

        assert_eq!(restored, omission);
        assert_eq!(
            SaliencePolicy::default(),
            SaliencePolicy::PriorityWithNeedBoost
        );
        assert_eq!(
            OmissionReason::SalienceBelowFloor {
                policy: SaliencePolicy::default()
            },
            OmissionReason::SalienceBelowFloor {
                policy: SaliencePolicy::PriorityWithNeedBoost
            }
        );
        assert_eq!(
            AgentBeliefStore::new().observation_omission_log,
            ObservationOmissionLog::default()
        );
    }

    #[test]
    fn belief_types_satisfy_component_and_serde_bounds() {
        assert_component_bounds::<AgentBeliefStore>();
        assert_component_bounds::<PerceptionProfile>();
        assert_component_bounds::<TellProfile>();
        assert_ordered_traits::<MismatchKind>();
        assert_serde_bounds::<BeliefConfidencePolicy>();
        assert_serde_bounds::<BelievedEntityState>();
        assert_serde_bounds::<BelievedEvidenceEntry>();
        assert_serde_bounds::<BelievedEvidenceState>();
        assert_serde_bounds::<MismatchKind>();
        assert_serde_bounds::<ObservationOmission>();
        assert_serde_bounds::<ObservationOmissionLog>();
        assert_serde_bounds::<OmissionReason>();
        assert_serde_bounds::<SaliencePolicy>();
        assert_serde_bounds::<SocialObservation>();
        assert_serde_bounds::<SocialObservationDetail>();
        assert_serde_bounds::<TellProfile>();
    }

    #[test]
    fn build_believed_entity_state_projects_authoritative_snapshot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let holder = world
            .create_agent("Holder", ControlSource::Ai, Tick(1))
            .unwrap();
        let bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(1))
            .unwrap();
        let water = world
            .create_item_lot(CommodityKind::Water, Quantity(3), Tick(1))
            .unwrap();
        let wound = sample_wound(4, 2);

        world.set_ground_location(holder, place).unwrap();
        world.set_ground_location(bread, place).unwrap();
        world.set_ground_location(water, place).unwrap();
        world.set_possessor(bread, holder).unwrap();
        world.set_possessor(water, holder).unwrap();
        world
            .insert_component_wound_list(
                holder,
                WoundList {
                    wounds: vec![wound.clone()],
                },
            )
            .unwrap();

        let snapshot = build_believed_entity_state(
            &world,
            holder,
            Tick(9),
            PerceptionSource::Report {
                from: entity(8),
                chain_len: 2,
            },
        )
        .unwrap();

        assert_eq!(snapshot.last_known_place, Some(place));
        assert_eq!(
            snapshot.last_known_inventory,
            BTreeMap::from([
                (CommodityKind::Bread, Quantity(2)),
                (CommodityKind::Water, Quantity(3)),
            ])
        );
        assert!(snapshot.alive);
        assert_eq!(snapshot.wounds, vec![wound]);
        assert_eq!(snapshot.believed_contention, None);
        assert_eq!(snapshot.last_observed_tick(), Some(Tick(9)));
        assert_eq!(
            snapshot.source,
            PerceptionSource::Report {
                from: entity(8),
                chain_len: 2,
            }
        );
    }

    #[test]
    fn build_observed_entity_snapshot_projects_authoritative_state_without_metadata() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let holder = world
            .create_agent("Holder", ControlSource::Ai, Tick(1))
            .unwrap();
        let bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(1))
            .unwrap();

        world.set_ground_location(holder, place).unwrap();
        world.set_ground_location(bread, place).unwrap();
        world.set_possessor(bread, holder).unwrap();

        let snapshot = build_observed_entity_snapshot(&world, holder).unwrap();

        assert_eq!(snapshot.believed_kind, Some(EntityKind::Agent));
        assert_eq!(snapshot.last_known_place, Some(place));
        assert_eq!(
            snapshot.last_known_inventory,
            BTreeMap::from([(CommodityKind::Bread, Quantity(2))])
        );
        assert!(snapshot.alive);
        assert!(snapshot.wounds.is_empty());
        assert_eq!(snapshot.courage, None); // no UtilityProfile set
        assert_eq!(snapshot.contention_state, None);
        assert_eq!(snapshot.evidence_state, None);
    }

    #[test]
    fn build_observed_entity_snapshot_projects_contention_state() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let target = world
            .create_agent("Target", ControlSource::Ai, Tick(1))
            .unwrap();
        let grantee = world
            .create_agent("Grantee", ControlSource::Ai, Tick(1))
            .unwrap();
        let waiter = world
            .create_agent("Waiter", ControlSource::Ai, Tick(1))
            .unwrap();

        for entity in [target, grantee, waiter] {
            world.set_ground_location(entity, place).unwrap();
        }
        world
            .insert_component_contention_queue(
                target,
                crate::ContentionQueue {
                    next_ordinal: 1,
                    waiting: BTreeMap::from([(
                        0,
                        crate::ContentionWaiter {
                            actor: waiter,
                            intended_action: ActionDefId(7),
                            queued_at: Tick(3),
                        },
                    )]),
                    granted: Some(crate::ContentionGrant {
                        actor: grantee,
                        intended_action: ActionDefId(6),
                        granted_at: Tick(4),
                        expires_at: Tick(9),
                    }),
                },
            )
            .unwrap();

        let snapshot = build_observed_entity_snapshot(&world, target).unwrap();
        assert_eq!(
            snapshot.contention_state,
            Some(BelievedContentionState {
                grant_holder: Some(grantee),
                queue_length: 1,
                observed_tick: Tick(0),
            })
        );

        let believed =
            snapshot.to_believed_entity_state(Tick(8), PerceptionSource::DirectObservation);
        assert_eq!(
            believed.believed_contention,
            Some(BelievedContentionState {
                grant_holder: Some(grantee),
                queue_length: 1,
                observed_tick: Tick(0),
            })
        );
    }

    #[test]
    fn build_observed_entity_snapshot_projects_evidence_state_for_places() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        world
            .insert_component_scene_evidence(
                place,
                SceneEvidence {
                    evidence: vec![crate::EvidenceEntry {
                        id: crate::EvidenceEntryId(0),
                        kind: EvidenceKind::DisturbanceMarker {
                            place,
                            kind: DisturbanceKind::WildernessRelief,
                            created_at: Tick(4),
                        },
                        created_at: Tick(4),
                        decay_ticks: 50,
                    }],
                    next_entry_id: 1,
                },
            )
            .unwrap();

        let snapshot = build_observed_entity_snapshot(&world, place).unwrap();
        assert_eq!(
            snapshot.evidence_state,
            Some(BelievedEvidenceState {
                entries: vec![BelievedEvidenceEntry {
                    kind: EvidenceKind::DisturbanceMarker {
                        place,
                        kind: DisturbanceKind::WildernessRelief,
                        created_at: Tick(4),
                    },
                    freshness: Tick(4),
                }],
                observed_tick: Tick(0),
            })
        );

        let believed =
            snapshot.to_believed_entity_state(Tick(9), PerceptionSource::DirectObservation);
        assert_eq!(
            believed.believed_evidence,
            Some(BelievedEvidenceState {
                entries: vec![BelievedEvidenceEntry {
                    kind: EvidenceKind::DisturbanceMarker {
                        place,
                        kind: DisturbanceKind::WildernessRelief,
                        created_at: Tick(4),
                    },
                    freshness: Tick(4),
                }],
                observed_tick: Tick(0),
            })
        );
    }

    #[test]
    fn build_observed_entity_snapshot_projects_artifact_state() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let issuer = world
            .create_agent("Issuer", ControlSource::Ai, Tick(1))
            .unwrap();
        let artifact = world.create_entity(EntityKind::SocialArtifact, Tick(1));

        world.set_ground_location(issuer, place).unwrap();
        world.set_ground_location(artifact, place).unwrap();
        world
            .insert_component_artifact_header(
                artifact,
                crate::ArtifactHeader {
                    kind: crate::ArtifactKind::Bounty,
                    issuer,
                    issuing_authority: None,
                    created_at: Tick(2),
                    expires_at: Some(Tick(7)),
                    state: crate::ArtifactState::Active,
                    jurisdiction: None,
                },
            )
            .unwrap();
        world
            .insert_component_bounty_terms(
                artifact,
                crate::BountyTerms {
                    target: crate::BountyTarget::EliminateEntity { target: issuer },
                    proof_requirement: crate::ProofRequirement::SelfReport,
                    reward_commodity: CommodityKind::Coin,
                    reward_quantity: Quantity(9),
                    reward_source: crate::RewardSource::PersonalFunds { issuer },
                    claim_place: place,
                },
            )
            .unwrap();

        let snapshot = build_observed_entity_snapshot(&world, artifact).unwrap();
        assert_eq!(
            snapshot.artifact_state,
            Some(BelievedArtifactState {
                kind: crate::ArtifactKind::Bounty,
                state: crate::ArtifactState::Active,
                issuer,
                expires_at: Some(Tick(7)),
                bounty_terms: Some(BelievedBountyTerms {
                    target: crate::BountyTarget::EliminateEntity { target: issuer },
                    reward_commodity: CommodityKind::Coin,
                    reward_quantity: Quantity(9),
                    claim_place: place,
                }),
                notice_topic: None,
                observed_tick: Tick(0),
            })
        );
    }

    #[test]
    fn build_believed_entity_state_projects_contention_with_current_tick() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let target = world
            .create_agent("Target", ControlSource::Ai, Tick(1))
            .unwrap();
        let grantee = world
            .create_agent("Grantee", ControlSource::Ai, Tick(1))
            .unwrap();

        for entity in [target, grantee] {
            world.set_ground_location(entity, place).unwrap();
        }
        world
            .insert_component_contention_queue(
                target,
                crate::ContentionQueue {
                    next_ordinal: 0,
                    waiting: BTreeMap::new(),
                    granted: Some(crate::ContentionGrant {
                        actor: grantee,
                        intended_action: ActionDefId(4),
                        granted_at: Tick(4),
                        expires_at: Tick(9),
                    }),
                },
            )
            .unwrap();

        let believed = build_believed_entity_state(
            &world,
            target,
            Tick(12),
            PerceptionSource::DirectObservation,
        )
        .unwrap();

        assert_eq!(
            believed.believed_contention,
            Some(BelievedContentionState {
                grant_holder: Some(grantee),
                queue_length: 0,
                observed_tick: Tick(12),
            })
        );
    }

    #[test]
    fn build_believed_entity_state_projects_artifact_with_current_tick() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let issuer = world
            .create_agent("Issuer", ControlSource::Ai, Tick(1))
            .unwrap();
        let office = world.create_office("Harbormaster", Tick(1)).unwrap();
        let artifact = world.create_entity(EntityKind::SocialArtifact, Tick(1));

        world.set_ground_location(issuer, place).unwrap();
        world.set_ground_location(office, place).unwrap();
        world.set_ground_location(artifact, place).unwrap();
        world
            .insert_component_artifact_header(
                artifact,
                crate::ArtifactHeader {
                    kind: crate::ArtifactKind::Notice,
                    issuer,
                    issuing_authority: None,
                    created_at: Tick(2),
                    expires_at: Some(Tick(8)),
                    state: crate::ArtifactState::Active,
                    jurisdiction: None,
                },
            )
            .unwrap();
        world
            .insert_component_notice_content(
                artifact,
                crate::NoticeContent {
                    topic: NoticeTopic::OfficeVacancy { office },
                },
            )
            .unwrap();

        let believed = build_believed_entity_state(
            &world,
            artifact,
            Tick(12),
            PerceptionSource::DirectObservation,
        )
        .unwrap();

        assert_eq!(
            believed.believed_artifact,
            Some(BelievedArtifactState {
                kind: crate::ArtifactKind::Notice,
                state: crate::ArtifactState::Active,
                issuer,
                expires_at: Some(Tick(8)),
                bounty_terms: None,
                notice_topic: Some(NoticeTopic::OfficeVacancy { office }),
                observed_tick: Tick(12),
            })
        );
    }

    #[test]
    fn build_observed_entity_snapshot_includes_lawfully_controlled_unpossessed_stock() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let member = world
            .create_agent("Holder", ControlSource::Ai, Tick(1))
            .unwrap();
        let faction = world.create_faction("River Pact", Tick(2)).unwrap();
        let bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(3), Tick(3))
            .unwrap();

        world.set_ground_location(member, place).unwrap();
        world.set_ground_location(bread, place).unwrap();
        world.set_owner(bread, faction).unwrap();
        world.add_member(member, faction).unwrap();

        let snapshot = build_observed_entity_snapshot(&world, member).unwrap();

        assert_eq!(
            snapshot.last_known_inventory,
            BTreeMap::from([(CommodityKind::Bread, Quantity(3))])
        );
    }

    #[test]
    fn build_observed_entity_snapshot_preserves_item_lot_self_quantity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(4), Tick(1))
            .unwrap();

        world.set_ground_location(bread, place).unwrap();

        let snapshot = build_observed_entity_snapshot(&world, bread).unwrap();

        assert_eq!(
            snapshot.last_known_inventory,
            BTreeMap::from([(CommodityKind::Bread, Quantity(4))])
        );
    }

    #[test]
    fn build_observed_entity_snapshot_captures_courage() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = world
            .create_agent("Brave", ControlSource::Ai, Tick(1))
            .unwrap();
        world.set_ground_location(agent, place).unwrap();

        let courage = Permille::new(600).unwrap();
        world
            .insert_component_utility_profile(
                agent,
                crate::UtilityProfile {
                    courage,
                    ..crate::UtilityProfile::default()
                },
            )
            .unwrap();

        let snapshot = build_observed_entity_snapshot(&world, agent).unwrap();
        assert_eq!(snapshot.believed_kind, Some(EntityKind::Agent));
        assert_eq!(snapshot.courage, Some(courage));

        // Verify it propagates through to_believed_entity_state
        let believed =
            snapshot.to_believed_entity_state(Tick(2), PerceptionSource::DirectObservation);
        assert_eq!(believed.believed_kind, Some(EntityKind::Agent));
        assert_eq!(believed.last_known_courage, Some(courage));
    }

    #[test]
    fn build_observed_entity_snapshot_captures_believed_kind_for_places_agents_and_item_lots() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = world
            .create_agent("Observer", ControlSource::Ai, Tick(1))
            .unwrap();
        let bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(1), Tick(1))
            .unwrap();

        world.set_ground_location(agent, place).unwrap();
        world.set_ground_location(bread, place).unwrap();

        assert_eq!(
            build_observed_entity_snapshot(&world, place)
                .unwrap()
                .believed_kind,
            Some(EntityKind::Place)
        );
        assert_eq!(
            build_observed_entity_snapshot(&world, agent)
                .unwrap()
                .believed_kind,
            Some(EntityKind::Agent)
        );
        assert_eq!(
            build_observed_entity_snapshot(&world, bread)
                .unwrap()
                .believed_kind,
            Some(EntityKind::ItemLot)
        );
    }

    #[test]
    fn refresh_entity_summary_from_claims_preserves_prior_believed_kind() {
        let subject = entity(66);
        let mut store = AgentBeliefStore::new();
        store.known_entities.insert(
            subject,
            BelievedEntityState {
                believed_kind: Some(EntityKind::Facility),
                last_known_place: Some(entity(10)),
                last_known_inventory: BTreeMap::new(),
                workstation_tag: None,
                resource_source: None,
                alive: true,
                wounds: Vec::new(),
                last_known_courage: None,
                believed_activity: None,
                believed_artifact: None,
                believed_contention: None,
                believed_evidence: None,
                ..BelievedEntityState::single_observation_defaults(
                    Tick(1),
                    PerceptionSource::DirectObservation,
                )
            },
        );
        store.entity_claims.insert(
            subject,
            vec![sample_claim(
                0,
                66,
                EntityBeliefAspect::Location,
                ClaimValue::Place(Some(entity(11))),
                PerceptionSource::DirectObservation,
                4,
                900,
            )],
        );

        store.refresh_entity_summary_from_claims(subject, Tick(4), &policy());

        assert_eq!(
            store.get_entity(&subject).unwrap().believed_kind,
            Some(EntityKind::Facility)
        );
    }

    #[test]
    fn record_entity_snapshot_claims_preserves_snapshot_believed_kind_without_prior_summary() {
        let subject = entity(67);
        let mut store = AgentBeliefStore::new();
        let snapshot = BelievedEntityState {
            believed_kind: Some(EntityKind::Agent),
            last_known_place: Some(entity(10)),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                Tick(4),
                PerceptionSource::DirectObservation,
            )
        };

        store.record_entity_snapshot_claims(
            subject,
            &snapshot,
            None,
            Tick(4),
            Some(Tick(4)),
            5,
            &policy(),
        );

        assert_eq!(
            store.get_entity(&subject).unwrap().believed_kind,
            Some(EntityKind::Agent)
        );
    }

    #[test]
    fn build_believed_entity_state_handles_dead_or_missing_entities() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let dead = world
            .create_agent("Dead", ControlSource::Ai, Tick(1))
            .unwrap();

        world.set_ground_location(dead, place).unwrap();
        world
            .insert_component_dead_at(
                dead,
                DeadAt {
                    tick: Tick(5),
                    cause: crate::DeathCause::CombatWounds,
                },
            )
            .unwrap();

        let dead_snapshot = build_believed_entity_state(
            &world,
            dead,
            Tick(7),
            PerceptionSource::Rumor { chain_len: 1 },
        )
        .unwrap();
        assert!(!dead_snapshot.alive);
        assert_eq!(dead_snapshot.last_known_place, Some(place));
        assert_eq!(
            dead_snapshot.source,
            PerceptionSource::Rumor { chain_len: 1 }
        );

        assert_eq!(
            build_believed_entity_state(
                &world,
                entity(999),
                Tick(7),
                PerceptionSource::DirectObservation,
            ),
            None
        );
    }

    #[test]
    fn update_believed_activity_changes_known_entity() {
        let mut store = AgentBeliefStore::new();
        let id = entity(1);
        store.known_entities.insert(id, sample_state(5, 3));

        let activity = BelievedActivity {
            action_domain: ActionDomain::Production,
            target: Some(entity(10)),
            observed_tick: Tick(6),
        };

        assert!(store.update_believed_activity(&id, Some(activity.clone()), Tick(6), &policy()));
        assert_eq!(
            store.get_entity(&id).unwrap().believed_activity,
            Some(activity.clone())
        );
        assert!(store.entity_claims.get(&id).unwrap().iter().any(|claim| {
            claim.aspect == EntityBeliefAspect::Activity
                && claim.value == ClaimValue::Activity(Some(activity.clone()))
        }));
    }

    #[test]
    fn update_believed_activity_noop_when_same() {
        let mut store = AgentBeliefStore::new();
        let id = entity(1);
        let mut state = sample_state(5, 3);
        let activity = BelievedActivity {
            action_domain: ActionDomain::Trade,
            target: None,
            observed_tick: Tick(5),
        };
        state.believed_activity = Some(activity.clone());
        store.known_entities.insert(id, state);

        assert!(!store.update_believed_activity(&id, Some(activity), Tick(5), &policy()));
    }

    #[test]
    fn update_believed_activity_unknown_entity() {
        let mut store = AgentBeliefStore::new();
        assert!(!store.update_believed_activity(&entity(99), None, Tick(5), &policy()));
    }

    #[test]
    fn clear_believed_activity_returns_true_when_some() {
        let mut store = AgentBeliefStore::new();
        let id = entity(2);
        let mut state = sample_state(5, 1);
        state.believed_activity = Some(BelievedActivity {
            action_domain: ActionDomain::Production,
            target: None,
            observed_tick: Tick(5),
        });
        store.known_entities.insert(id, state);

        assert!(store.clear_believed_activity(&id, Tick(6), &policy()));
        assert_eq!(store.get_entity(&id).unwrap().believed_activity, None);
        assert!(store.entity_claims.get(&id).unwrap().iter().any(|claim| {
            claim.aspect == EntityBeliefAspect::Activity
                && claim.value == ClaimValue::Activity(None)
        }));
    }

    #[test]
    fn clear_believed_activity_returns_false_when_none() {
        let mut store = AgentBeliefStore::new();
        let id = entity(3);
        store.known_entities.insert(id, sample_state(5, 1));

        assert!(!store.clear_believed_activity(&id, Tick(6), &policy()));
    }

    #[test]
    fn clear_believed_activity_unknown_entity() {
        let mut store = AgentBeliefStore::new();
        assert!(!store.clear_believed_activity(&entity(99), Tick(6), &policy()));
    }

    #[test]
    fn update_departure_projection_updates_known_entity() {
        let mut store = AgentBeliefStore::new();
        let id = entity(4);
        let destination = entity(8);
        store.known_entities.insert(id, sample_state(5, 1));

        assert!(store.update_departure_projection(&id, destination, Tick(9), &policy()));

        let belief = store.get_entity(&id).unwrap();
        assert_eq!(belief.last_known_place, Some(destination));
        assert_eq!(belief.last_observed_tick(), Some(Tick(9)));
        assert_eq!(belief.source, PerceptionSource::DirectObservation);
        assert!(store.entity_claims.get(&id).unwrap().iter().any(|claim| {
            claim.aspect == EntityBeliefAspect::Location
                && claim.value == ClaimValue::Place(Some(destination))
        }));
    }

    #[test]
    fn update_departure_projection_unknown_entity() {
        let mut store = AgentBeliefStore::new();
        assert!(!store.update_departure_projection(&entity(99), entity(8), Tick(9), &policy(),));
    }

    #[test]
    fn import_entity_snapshot_records_report_claims_and_survives_prune() {
        let mut store = AgentBeliefStore::new();
        let id = entity(12);
        let evidence = BelievedEvidenceState {
            entries: vec![BelievedEvidenceEntry {
                kind: EvidenceKind::DisturbanceMarker {
                    place: entity(20),
                    kind: crate::DisturbanceKind::WildernessRelief,
                    created_at: Tick(6),
                },
                freshness: Tick(6),
            }],
            observed_tick: Tick(6),
        };
        let mut snapshot = sample_state(6, 2);
        snapshot.source = PerceptionSource::Report {
            from: entity(30),
            chain_len: 1,
        };
        snapshot.believed_evidence = Some(evidence.clone());

        store.import_entity_snapshot(id, &snapshot, Tick(8), &policy());
        store.prune_decayed_beliefs(
            &profile(100, 50, 5),
            Tick(9),
            &HomeostaticNeeds::new_sated(),
        );

        let summary = store.get_entity(&id).unwrap();
        assert_eq!(summary.believed_evidence, Some(evidence));
        assert_eq!(
            summary.source,
            PerceptionSource::Report {
                from: entity(30),
                chain_len: 1,
            }
        );
        assert!(
            store
                .entity_claims
                .get(&id)
                .unwrap()
                .iter()
                .any(|claim| claim.aspect == EntityBeliefAspect::Evidence)
        );
    }

    // ── BeliefStoreDiff tests ──────────────────────────────────────────

    use super::BeliefStoreDiff;

    fn make_believed_entity(tick: Tick) -> BelievedEntityState {
        BelievedEntityState {
            believed_kind: None,
            last_known_place: Some(entity(100)),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: vec![],
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                tick,
                PerceptionSource::DirectObservation,
            )
        }
    }

    fn make_social_observation(tick: Tick) -> SocialObservation {
        SocialObservation {
            detail: SocialObservationDetail::WitnessedCooperation {
                actor: entity(1),
                counterpart: entity(2),
            },
            place: entity(10),
            observed_tick: tick,
            source: PerceptionSource::DirectObservation,
        }
    }

    fn make_observation_omission(tick: Tick) -> ObservationOmission {
        ObservationOmission {
            omitted_entity: entity(tick.0 as u32),
            reason: OmissionReason::OverBudget {
                budget: 12,
                candidates_seen: 30,
            },
            observed_tick: tick,
        }
    }

    fn make_tell_key(counterparty_slot: u32) -> TellMemoryKey {
        TellMemoryKey {
            counterparty: entity(counterparty_slot),
            topic: TellTopic::EntityBelief { subject: entity(1) },
        }
    }

    fn make_told_memory(tick: Tick) -> ToldBeliefMemory {
        ToldBeliefMemory {
            shared_state: SharedTellState::SocialObservation(make_social_observation(tick)),
            told_tick: tick,
        }
    }

    fn make_heard_memory(tick: Tick) -> HeardBeliefMemory {
        HeardBeliefMemory {
            heard_state: SharedTellState::SocialObservation(make_social_observation(tick)),
            heard_tick: tick,
            disposition: HeardBeliefDisposition::Accepted,
        }
    }

    fn make_ask_witness_key(counterparty_slot: u32) -> AskWitnessMemoryKey {
        AskWitnessMemoryKey {
            counterparty: entity(counterparty_slot),
            topic_entity: Some(entity(1)),
            topic_commodity: None,
        }
    }

    fn make_ask_witness_memory(tick: Tick) -> AskWitnessMemory {
        AskWitnessMemory { asked_tick: tick }
    }

    fn make_place_visit_record(tick: Tick) -> PlaceVisitRecord {
        PlaceVisitRecord {
            ticks_present: tick.0 as u32,
            last_arrival_tick: tick,
            visit_count: (tick.0 as u16).max(1),
        }
    }

    fn make_institutional_key(office_slot: u32) -> InstitutionalBeliefKey {
        InstitutionalBeliefKey::OfficeHolderOf {
            office: entity(office_slot),
        }
    }

    fn make_institutional_claim(tick: Tick) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::OfficeHolder {
                office: entity(50),
                holder: Some(entity(51)),
                effective_tick: tick,
            },
            source: InstitutionalKnowledgeSource::DirectObservation,
            learned_tick: tick,
            learned_at: Some(entity(100)),
        }
    }

    fn make_entity_claim(subject_slot: u32, tick: Tick) -> EntityBeliefClaim {
        EntityBeliefClaim {
            claim_id: ClaimId(tick.0),
            subject: entity(subject_slot),
            aspect: EntityBeliefAspect::Alive,
            value: ClaimValue::Bool(true),
            source: PerceptionSource::DirectObservation,
            acquired_tick: tick,
            claimed_event_tick: Some(tick),
            confidence: Permille::new(800).unwrap(),
            refuted_at_tick: None,
        }
    }

    #[test]
    fn belief_store_diff_empty_stores() {
        let a = AgentBeliefStore::new();
        let b = AgentBeliefStore::new();
        let diff = BeliefStoreDiff::compute(&a, &b);
        assert!(diff.is_empty());
        assert_eq!(diff, BeliefStoreDiff::default());
        assert_eq!(diff.apply(&a), b);
    }

    #[test]
    fn belief_store_diff_identity() {
        let mut store = AgentBeliefStore::new();
        store
            .known_entities
            .insert(entity(1), make_believed_entity(Tick(10)));
        store
            .social_observations
            .push(make_social_observation(Tick(5)));
        store
            .told_beliefs
            .insert(make_tell_key(2), make_told_memory(Tick(3)));
        store.next_claim_id = ClaimId(42);

        let diff = BeliefStoreDiff::compute(&store, &store);
        assert!(diff.is_empty());
        assert_eq!(diff.apply(&store), store);
    }

    #[test]
    fn belief_store_diff_roundtrip_known_entities() {
        let mut before = AgentBeliefStore::new();
        before
            .known_entities
            .insert(entity(1), make_believed_entity(Tick(1)));
        before
            .known_entities
            .insert(entity(2), make_believed_entity(Tick(2)));

        let mut after = before.clone();
        after.known_entities.remove(&entity(1)); // removed
        after
            .known_entities
            .insert(entity(3), make_believed_entity(Tick(3))); // added
        after
            .known_entities
            .insert(entity(2), make_believed_entity(Tick(20))); // changed

        let diff = BeliefStoreDiff::compute(&before, &after);
        assert!(!diff.is_empty());
        assert_eq!(diff.known_entities_removed, vec![entity(1)]);
        assert_eq!(diff.known_entities_set.len(), 2); // entity(2) changed + entity(3) added
        assert_eq!(diff.apply(&before), after);
    }

    #[test]
    fn belief_store_diff_roundtrip_social_observations() {
        let mut before = AgentBeliefStore::new();
        before
            .social_observations
            .push(make_social_observation(Tick(1)));
        before
            .social_observations
            .push(make_social_observation(Tick(2)));
        before
            .social_observations
            .push(make_social_observation(Tick(3)));

        // Evict first entry, add two new ones.
        let mut after = before.clone();
        after.social_observations.remove(0);
        after
            .social_observations
            .push(make_social_observation(Tick(4)));
        after
            .social_observations
            .push(make_social_observation(Tick(5)));

        let diff = BeliefStoreDiff::compute(&before, &after);
        assert_eq!(diff.social_observations_removed_count, 1);
        assert_eq!(diff.social_observations_added.len(), 2);
        assert_eq!(diff.apply(&before), after);
    }

    #[test]
    fn belief_store_diff_roundtrip_observation_omission_log() {
        let mut before = AgentBeliefStore::new();
        before
            .observation_omission_log
            .entries
            .push_back(make_observation_omission(Tick(1)));
        before
            .observation_omission_log
            .entries
            .push_back(make_observation_omission(Tick(2)));
        before
            .observation_omission_log
            .entries
            .push_back(make_observation_omission(Tick(3)));

        let mut after = before.clone();
        after.observation_omission_log.entries.pop_front();
        after
            .observation_omission_log
            .entries
            .push_back(make_observation_omission(Tick(4)));
        after
            .observation_omission_log
            .entries
            .push_back(make_observation_omission(Tick(5)));

        let diff = BeliefStoreDiff::compute(&before, &after);
        assert_eq!(diff.omission_log_removed_count, 1);
        assert_eq!(diff.omission_log_added.len(), 2);
        assert_eq!(diff.apply(&before), after);
    }

    #[test]
    fn belief_store_diff_roundtrip_social_observation_same_detail_replacement() {
        let mut before = AgentBeliefStore::new();
        before
            .social_observations
            .push(make_social_observation(Tick(1)));
        before.social_observations.push(SocialObservation {
            detail: SocialObservationDetail::CoPresence { other: entity(7) },
            ..make_social_observation(Tick(2))
        });

        let mut after = before.clone();
        after.social_observations[0] = SocialObservation {
            place: entity(99),
            observed_tick: Tick(5),
            source: PerceptionSource::Report {
                from: entity(8),
                chain_len: 1,
            },
            ..before.social_observations[0]
        };

        let diff = BeliefStoreDiff::compute(&before, &after);
        assert_eq!(diff.apply(&before), after);
    }

    #[test]
    fn belief_store_diff_roundtrip_told_heard_beliefs() {
        let mut before = AgentBeliefStore::new();
        before
            .told_beliefs
            .insert(make_tell_key(1), make_told_memory(Tick(1)));
        before
            .heard_beliefs
            .insert(make_tell_key(2), make_heard_memory(Tick(2)));

        let mut after = before.clone();
        after.told_beliefs.remove(&make_tell_key(1)); // removed
        after
            .told_beliefs
            .insert(make_tell_key(3), make_told_memory(Tick(3))); // added
        after
            .heard_beliefs
            .insert(make_tell_key(2), make_heard_memory(Tick(20))); // changed

        let diff = BeliefStoreDiff::compute(&before, &after);
        assert_eq!(diff.told_beliefs_removed.len(), 1);
        assert_eq!(diff.told_beliefs_set.len(), 1);
        assert_eq!(diff.heard_beliefs_set.len(), 1);
        assert!(diff.heard_beliefs_removed.is_empty());
        assert_eq!(diff.apply(&before), after);
    }

    #[test]
    fn belief_store_diff_roundtrip_asked_witnesses() {
        let mut before = AgentBeliefStore::new();
        before
            .asked_witnesses
            .insert(make_ask_witness_key(1), make_ask_witness_memory(Tick(1)));

        let mut after = before.clone();
        after.asked_witnesses.remove(&make_ask_witness_key(1));
        after
            .asked_witnesses
            .insert(make_ask_witness_key(2), make_ask_witness_memory(Tick(2)));

        let diff = BeliefStoreDiff::compute(&before, &after);
        assert_eq!(diff.asked_witnesses_removed.len(), 1);
        assert_eq!(diff.asked_witnesses_set.len(), 1);
        assert_eq!(diff.apply(&before), after);
    }

    #[test]
    fn belief_store_diff_roundtrip_place_visits() {
        let mut before = AgentBeliefStore::new();
        before
            .place_visits
            .insert(entity(1), make_place_visit_record(Tick(1)));

        let mut after = before.clone();
        after.place_visits.remove(&entity(1));
        after
            .place_visits
            .insert(entity(2), make_place_visit_record(Tick(2)));

        let diff = BeliefStoreDiff::compute(&before, &after);
        assert_eq!(diff.place_visits_removed.len(), 1);
        assert_eq!(diff.place_visits_set.len(), 1);
        assert_eq!(diff.apply(&before), after);
    }

    #[test]
    fn belief_store_diff_roundtrip_entity_claims() {
        let mut before = AgentBeliefStore::new();
        before
            .entity_claims
            .insert(entity(1), vec![make_entity_claim(1, Tick(1))]);

        let mut after = before.clone();
        after.entity_claims.remove(&entity(1));
        after
            .entity_claims
            .insert(entity(2), vec![make_entity_claim(2, Tick(2))]);

        let diff = BeliefStoreDiff::compute(&before, &after);
        assert_eq!(diff.entity_claims_removed.len(), 1);
        assert_eq!(diff.entity_claims_set.len(), 1);
        assert_eq!(diff.apply(&before), after);
    }

    #[test]
    fn belief_store_diff_roundtrip_institutional_beliefs() {
        let mut before = AgentBeliefStore::new();
        before.institutional_beliefs.insert(
            make_institutional_key(50),
            vec![make_institutional_claim(Tick(1))],
        );

        let mut after = before.clone();
        after
            .institutional_beliefs
            .remove(&make_institutional_key(50));
        after.institutional_beliefs.insert(
            make_institutional_key(51),
            vec![make_institutional_claim(Tick(2))],
        );

        let diff = BeliefStoreDiff::compute(&before, &after);
        assert_eq!(diff.institutional_beliefs_removed.len(), 1);
        assert_eq!(diff.institutional_beliefs_set.len(), 1);
        assert_eq!(diff.apply(&before), after);
    }

    #[test]
    fn belief_store_diff_roundtrip_next_claim_id() {
        let mut before = AgentBeliefStore::new();
        before.next_claim_id = ClaimId(10);

        let mut after = before.clone();
        after.next_claim_id = ClaimId(15);

        let diff = BeliefStoreDiff::compute(&before, &after);
        assert_eq!(diff.next_claim_id, Some(ClaimId(15)));
        assert_eq!(diff.apply(&before), after);
    }

    #[test]
    fn belief_store_diff_roundtrip_mixed_mutations() {
        let mut before = AgentBeliefStore::new();
        before.next_claim_id = ClaimId(5);
        before
            .known_entities
            .insert(entity(1), make_believed_entity(Tick(1)));
        before
            .social_observations
            .push(make_social_observation(Tick(1)));
        before
            .told_beliefs
            .insert(make_tell_key(1), make_told_memory(Tick(1)));
        before
            .heard_beliefs
            .insert(make_tell_key(2), make_heard_memory(Tick(2)));
        before
            .asked_witnesses
            .insert(make_ask_witness_key(1), make_ask_witness_memory(Tick(1)));
        before
            .place_visits
            .insert(entity(1), make_place_visit_record(Tick(1)));
        before
            .entity_claims
            .insert(entity(1), vec![make_entity_claim(1, Tick(1))]);
        before.institutional_beliefs.insert(
            make_institutional_key(50),
            vec![make_institutional_claim(Tick(1))],
        );

        let mut after = before.clone();
        after.next_claim_id = ClaimId(10);
        after
            .known_entities
            .insert(entity(2), make_believed_entity(Tick(5)));
        after
            .social_observations
            .push(make_social_observation(Tick(5)));
        after.told_beliefs.remove(&make_tell_key(1));
        after
            .heard_beliefs
            .insert(make_tell_key(3), make_heard_memory(Tick(5)));
        after
            .asked_witnesses
            .insert(make_ask_witness_key(3), make_ask_witness_memory(Tick(5)));
        after
            .place_visits
            .insert(entity(2), make_place_visit_record(Tick(5)));
        after
            .entity_claims
            .insert(entity(2), vec![make_entity_claim(2, Tick(5))]);
        after.institutional_beliefs.insert(
            make_institutional_key(51),
            vec![make_institutional_claim(Tick(5))],
        );

        let diff = BeliefStoreDiff::compute(&before, &after);
        assert!(!diff.is_empty());
        assert_eq!(diff.apply(&before), after);
    }

    #[test]
    fn belief_store_diff_social_observations_full_replacement() {
        let mut before = AgentBeliefStore::new();
        before
            .social_observations
            .push(make_social_observation(Tick(1)));
        before
            .social_observations
            .push(make_social_observation(Tick(2)));

        // Completely different observations.
        let mut after = AgentBeliefStore::new();
        after
            .social_observations
            .push(make_social_observation(Tick(10)));
        after
            .social_observations
            .push(make_social_observation(Tick(11)));

        let diff = BeliefStoreDiff::compute(&before, &after);
        assert_eq!(diff.apply(&before), after);
    }

    #[test]
    fn belief_store_diff_serialization_roundtrip() {
        let mut before = AgentBeliefStore::new();
        before
            .known_entities
            .insert(entity(1), make_believed_entity(Tick(1)));
        before.next_claim_id = ClaimId(5);

        let mut after = before.clone();
        after
            .known_entities
            .insert(entity(2), make_believed_entity(Tick(2)));
        after.next_claim_id = ClaimId(10);

        let diff = BeliefStoreDiff::compute(&before, &after);
        let bytes = bincode::serialize(&diff).expect("serialize");
        let restored: BeliefStoreDiff = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(diff, restored);
        assert_eq!(restored.apply(&before), after);
    }
}
