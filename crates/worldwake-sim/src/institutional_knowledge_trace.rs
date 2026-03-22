//! Append-only authoritative institutional knowledge trace.
//!
//! Records effective institutional belief transitions caused by explicit
//! knowledge-acquisition actions such as `consult_record`.

use std::collections::BTreeMap;
use worldwake_core::{
    AgentBeliefStore, EntityId, InstitutionalBeliefKey, InstitutionalBeliefRead, RecordData,
    RecordEntryId, Tick,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstitutionalKnowledgeTraceEvent {
    pub tick: Tick,
    pub sequence_in_tick: u32,
    pub actor: EntityId,
    pub source: InstitutionalKnowledgeTraceSource,
    pub transitions: Vec<InstitutionalBeliefTransitionTrace>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstitutionalKnowledgeTraceSource {
    RecordConsultation {
        record: EntityId,
        home_place: EntityId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstitutionalBeliefTransitionTrace {
    pub key: InstitutionalBeliefKey,
    pub source_entry_ids: Vec<RecordEntryId>,
    pub previous: InstitutionalBeliefReadSummary,
    pub new: InstitutionalBeliefReadSummary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstitutionalBeliefReadSummary {
    Unknown,
    OfficeHolderCertain { holder: Option<EntityId> },
    OfficeHolderConflicted { holders: Vec<Option<EntityId>> },
    FactionMembershipClaims { claims: Vec<FactionMembershipClaimSummary> },
    SupportDeclarationCertain { candidate: Option<EntityId> },
    SupportDeclarationConflicted { candidates: Vec<Option<EntityId>> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactionMembershipClaimSummary {
    pub member: EntityId,
    pub active: bool,
}

impl InstitutionalKnowledgeTraceEvent {
    #[must_use]
    pub fn summary(&self) -> String {
        match &self.source {
            InstitutionalKnowledgeTraceSource::RecordConsultation { record, home_place } => {
                format!(
                    "tick {} seq {}: {} learned {} institutional transitions from consulted record {} at {}",
                    self.tick.0,
                    self.sequence_in_tick,
                    self.actor,
                    self.transitions.len(),
                    record,
                    home_place
                )
            }
        }
    }
}

pub struct InstitutionalKnowledgeTraceSink {
    events: Vec<InstitutionalKnowledgeTraceEvent>,
    next_sequence_in_tick: BTreeMap<Tick, u32>,
}

impl InstitutionalKnowledgeTraceSink {
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_sequence_in_tick: BTreeMap::new(),
        }
    }

    pub fn record(&mut self, mut event: InstitutionalKnowledgeTraceEvent) {
        let sequence_in_tick = self.next_sequence_in_tick.entry(event.tick).or_insert(0);
        event.sequence_in_tick = *sequence_in_tick;
        *sequence_in_tick = sequence_in_tick
            .checked_add(1)
            .expect("institutional knowledge trace per-tick sequence overflowed");
        self.events.push(event);
    }

    #[must_use]
    pub fn events(&self) -> &[InstitutionalKnowledgeTraceEvent] {
        &self.events
    }

    #[must_use]
    pub fn events_for(&self, actor: EntityId) -> Vec<&InstitutionalKnowledgeTraceEvent> {
        self.events.iter().filter(|event| event.actor == actor).collect()
    }

    #[must_use]
    pub fn events_at(&self, tick: Tick) -> Vec<&InstitutionalKnowledgeTraceEvent> {
        self.events.iter().filter(|event| event.tick == tick).collect()
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.next_sequence_in_tick.clear();
    }

    pub fn dump_actor(&self, actor: EntityId) {
        let events = self.events_for(actor);
        if events.is_empty() {
            eprintln!("[InstitutionalKnowledgeTrace] No events for {actor}");
            return;
        }
        eprintln!(
            "[InstitutionalKnowledgeTrace] {} events for {actor}:",
            events.len()
        );
        for event in events {
            eprintln!("  {}", event.summary());
        }
    }
}

impl Default for InstitutionalKnowledgeTraceSink {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
pub fn build_record_consultation_trace_event(
    tick: Tick,
    actor: EntityId,
    record: EntityId,
    record_data: &RecordData,
    before: &AgentBeliefStore,
    after: &AgentBeliefStore,
) -> Option<InstitutionalKnowledgeTraceEvent> {
    let consulted_entries = record_data
        .entries_newest_first()
        .take(record_data.max_entries_per_consult as usize);
    let mut entry_ids_by_key = BTreeMap::<InstitutionalBeliefKey, Vec<RecordEntryId>>::new();

    for entry in consulted_entries {
        entry_ids_by_key
            .entry(institutional_belief_key(entry.claim))
            .or_default()
            .push(entry.entry_id);
    }

    let transitions = entry_ids_by_key
        .into_iter()
        .filter_map(|(key, source_entry_ids)| {
            let previous = summarize_institutional_read(before, &key);
            let new = summarize_institutional_read(after, &key);
            (previous != new).then_some(InstitutionalBeliefTransitionTrace {
                key,
                source_entry_ids,
                previous,
                new,
            })
        })
        .collect::<Vec<_>>();

    (!transitions.is_empty()).then_some(InstitutionalKnowledgeTraceEvent {
        tick,
        sequence_in_tick: 0,
        actor,
        source: InstitutionalKnowledgeTraceSource::RecordConsultation {
            record,
            home_place: record_data.home_place,
        },
        transitions,
    })
}

#[must_use]
pub fn summarize_institutional_read(
    store: &AgentBeliefStore,
    key: &InstitutionalBeliefKey,
) -> InstitutionalBeliefReadSummary {
    match *key {
        InstitutionalBeliefKey::OfficeHolderOf { office } => {
            match store.believed_office_holder(office) {
                InstitutionalBeliefRead::Unknown => InstitutionalBeliefReadSummary::Unknown,
                InstitutionalBeliefRead::Certain(holder) => {
                    InstitutionalBeliefReadSummary::OfficeHolderCertain { holder }
                }
                InstitutionalBeliefRead::Conflicted(holders) => {
                    InstitutionalBeliefReadSummary::OfficeHolderConflicted { holders }
                }
            }
        }
        InstitutionalBeliefKey::FactionMembersOf { faction } => {
            let mut claims = store
                .institutional_beliefs
                .get(&InstitutionalBeliefKey::FactionMembersOf { faction })
                .into_iter()
                .flatten()
                .filter_map(|belief| match belief.claim {
                    worldwake_core::InstitutionalClaim::FactionMembership {
                        faction: claim_faction,
                        member,
                        active,
                        ..
                    } if claim_faction == faction => {
                        Some(FactionMembershipClaimSummary { member, active })
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            claims.sort_by_key(|claim| (claim.member, claim.active));
            if claims.is_empty() {
                InstitutionalBeliefReadSummary::Unknown
            } else {
                InstitutionalBeliefReadSummary::FactionMembershipClaims { claims }
            }
        }
        InstitutionalBeliefKey::SupportFor { supporter, office } => {
            match store.believed_support_declaration(office, supporter) {
                InstitutionalBeliefRead::Unknown => InstitutionalBeliefReadSummary::Unknown,
                InstitutionalBeliefRead::Certain(candidate) => {
                    InstitutionalBeliefReadSummary::SupportDeclarationCertain { candidate }
                }
                InstitutionalBeliefRead::Conflicted(candidates) => {
                    InstitutionalBeliefReadSummary::SupportDeclarationConflicted { candidates }
                }
            }
        }
    }
}

fn institutional_belief_key(claim: worldwake_core::InstitutionalClaim) -> InstitutionalBeliefKey {
    match claim {
        worldwake_core::InstitutionalClaim::OfficeHolder { office, .. } => {
            InstitutionalBeliefKey::OfficeHolderOf { office }
        }
        worldwake_core::InstitutionalClaim::FactionMembership { faction, .. } => {
            InstitutionalBeliefKey::FactionMembersOf { faction }
        }
        worldwake_core::InstitutionalClaim::SupportDeclaration {
            supporter, office, ..
        } => InstitutionalBeliefKey::SupportFor { supporter, office },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_core::{
        BelievedInstitutionalClaim, InstitutionalClaim, InstitutionalKnowledgeSource,
        PerceptionProfile, Permille,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn profile() -> PerceptionProfile {
        PerceptionProfile {
            institutional_memory_capacity: 20,
            consultation_speed_factor: Permille::new(500).unwrap(),
            ..PerceptionProfile::default()
        }
    }

    fn record_claim(
        store: &mut AgentBeliefStore,
        key: InstitutionalBeliefKey,
        claim: InstitutionalClaim,
        record: EntityId,
        entry_id: u32,
    ) {
        store.record_institutional_belief(
            key,
            BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record,
                    entry_id: RecordEntryId(u64::from(entry_id)),
                },
                learned_tick: Tick(5),
                learned_at: Some(entity(99)),
            },
            &profile(),
        );
    }

    #[test]
    fn build_record_consultation_trace_event_reports_semantic_transition() {
        let actor = entity(1);
        let office = entity(2);
        let record = entity(3);
        let key = InstitutionalBeliefKey::OfficeHolderOf { office };
        let before = AgentBeliefStore::default();
        let mut after = AgentBeliefStore::default();
        record_claim(
            &mut after,
            key,
            InstitutionalClaim::OfficeHolder {
                office,
                holder: None,
                effective_tick: Tick(0),
            },
            record,
            0,
        );
        let record_data = RecordData {
            record_kind: worldwake_core::RecordKind::OfficeRegister,
            home_place: entity(10),
            issuer: actor,
            consultation_ticks: 4,
            max_entries_per_consult: 1,
            entries: vec![worldwake_core::InstitutionalRecordEntry {
                entry_id: RecordEntryId(0),
                claim: InstitutionalClaim::OfficeHolder {
                    office,
                    holder: None,
                    effective_tick: Tick(0),
                },
                recorded_tick: Tick(0),
                supersedes: None,
            }],
            next_entry_id: 1,
        };

        let event = build_record_consultation_trace_event(
            Tick(5),
            actor,
            record,
            &record_data,
            &before,
            &after,
        )
        .expect("effective transition should emit a trace event");

        assert_eq!(
            event.transitions,
            vec![InstitutionalBeliefTransitionTrace {
                key,
                source_entry_ids: vec![RecordEntryId(0)],
                previous: InstitutionalBeliefReadSummary::Unknown,
                new: InstitutionalBeliefReadSummary::OfficeHolderCertain { holder: None },
            }]
        );
    }

    #[test]
    fn build_record_consultation_trace_event_suppresses_no_effective_transition() {
        let actor = entity(1);
        let office = entity(2);
        let record = entity(3);
        let key = InstitutionalBeliefKey::OfficeHolderOf { office };
        let claim = InstitutionalClaim::OfficeHolder {
            office,
            holder: None,
            effective_tick: Tick(0),
        };
        let mut before = AgentBeliefStore::default();
        let mut after = AgentBeliefStore::default();
        record_claim(&mut before, key, claim, record, 0);
        record_claim(&mut after, key, claim, record, 0);
        record_claim(&mut after, key, claim, record, 1);

        let record_data = RecordData {
            record_kind: worldwake_core::RecordKind::OfficeRegister,
            home_place: entity(10),
            issuer: actor,
            consultation_ticks: 4,
            max_entries_per_consult: 1,
            entries: vec![worldwake_core::InstitutionalRecordEntry {
                entry_id: RecordEntryId(1),
                claim,
                recorded_tick: Tick(1),
                supersedes: Some(RecordEntryId(0)),
            }],
            next_entry_id: 2,
        };

        assert_eq!(
            build_record_consultation_trace_event(
                Tick(5),
                actor,
                record,
                &record_data,
                &before,
                &after,
            ),
            None
        );
    }

    #[test]
    fn sink_assigns_sequence_per_tick() {
        let actor = entity(1);
        let record = entity(2);
        let mut sink = InstitutionalKnowledgeTraceSink::new();

        sink.record(InstitutionalKnowledgeTraceEvent {
            tick: Tick(7),
            sequence_in_tick: 99,
            actor,
            source: InstitutionalKnowledgeTraceSource::RecordConsultation {
                record,
                home_place: entity(10),
            },
            transitions: Vec::new(),
        });
        sink.record(InstitutionalKnowledgeTraceEvent {
            tick: Tick(7),
            sequence_in_tick: 99,
            actor,
            source: InstitutionalKnowledgeTraceSource::RecordConsultation {
                record,
                home_place: entity(10),
            },
            transitions: Vec::new(),
        });

        assert_eq!(sink.events()[0].sequence_in_tick, 0);
        assert_eq!(sink.events()[1].sequence_in_tick, 1);
    }

    #[test]
    fn summarize_institutional_read_reports_conflicted_support_values() {
        let office = entity(2);
        let supporter = entity(3);
        let record = entity(4);
        let key = InstitutionalBeliefKey::SupportFor { supporter, office };
        let mut store = AgentBeliefStore::default();
        record_claim(
            &mut store,
            key,
            InstitutionalClaim::SupportDeclaration {
                supporter,
                office,
                candidate: Some(entity(10)),
                effective_tick: Tick(0),
            },
            record,
            0,
        );
        record_claim(
            &mut store,
            key,
            InstitutionalClaim::SupportDeclaration {
                supporter,
                office,
                candidate: Some(entity(11)),
                effective_tick: Tick(1),
            },
            record,
            1,
        );

        assert_eq!(
            summarize_institutional_read(&store, &key),
            InstitutionalBeliefReadSummary::SupportDeclarationConflicted {
                candidates: vec![Some(entity(10)), Some(entity(11))],
            }
        );
    }
}
