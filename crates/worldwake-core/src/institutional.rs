use crate::{Component, EntityId, Tick};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RecordKind {
    OfficeRegister,
    FactionRoster,
    SupportLedger,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RecordEntryId(pub u64);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum InstitutionalClaim {
    OfficeHolder {
        office: EntityId,
        holder: Option<EntityId>,
        effective_tick: Tick,
    },
    FactionMembership {
        faction: EntityId,
        member: EntityId,
        active: bool,
        effective_tick: Tick,
    },
    SupportDeclaration {
        office: EntityId,
        supporter: EntityId,
        candidate: Option<EntityId>,
        effective_tick: Tick,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstitutionalRecordEntry {
    pub entry_id: RecordEntryId,
    pub claim: InstitutionalClaim,
    pub recorded_tick: Tick,
    pub supersedes: Option<RecordEntryId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecordData {
    pub record_kind: RecordKind,
    pub home_place: EntityId,
    pub issuer: EntityId,
    pub consultation_ticks: u32,
    pub max_entries_per_consult: u32,
    pub entries: Vec<InstitutionalRecordEntry>,
    pub next_entry_id: u64,
}

impl Component for RecordData {}

impl RecordData {
    pub fn append_entry(
        &mut self,
        claim: InstitutionalClaim,
        recorded_tick: Tick,
    ) -> RecordEntryId {
        let entry_id = RecordEntryId(self.next_entry_id);
        self.next_entry_id += 1;
        self.entries.push(InstitutionalRecordEntry {
            entry_id,
            claim,
            recorded_tick,
            supersedes: None,
        });
        entry_id
    }

    pub fn supersede_entry(
        &mut self,
        old_id: RecordEntryId,
        new_claim: InstitutionalClaim,
        recorded_tick: Tick,
    ) -> Result<RecordEntryId, InstitutionalRecordError> {
        if !self.entries.iter().any(|entry| entry.entry_id == old_id) {
            return Err(InstitutionalRecordError::EntryNotFound(old_id));
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.supersedes == Some(old_id))
        {
            return Err(InstitutionalRecordError::EntryAlreadySuperseded(old_id));
        }

        let entry_id = RecordEntryId(self.next_entry_id);
        self.next_entry_id += 1;
        self.entries.push(InstitutionalRecordEntry {
            entry_id,
            claim: new_claim,
            recorded_tick,
            supersedes: Some(old_id),
        });
        Ok(entry_id)
    }

    pub fn entries_newest_first(&self) -> impl Iterator<Item = &InstitutionalRecordEntry> {
        self.entries.iter().rev()
    }

    pub fn active_entries(&self) -> Vec<&InstitutionalRecordEntry> {
        let superseded = self
            .entries
            .iter()
            .filter_map(|entry| entry.supersedes)
            .collect::<BTreeSet<_>>();

        self.entries
            .iter()
            .filter(|entry| !superseded.contains(&entry.entry_id))
            .collect()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum InstitutionalRecordError {
    EntryNotFound(RecordEntryId),
    EntryAlreadySuperseded(RecordEntryId),
}

impl fmt::Display for InstitutionalRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntryNotFound(entry_id) => write!(f, "record entry not found: {}", entry_id.0),
            Self::EntryAlreadySuperseded(entry_id) => {
                write!(f, "record entry already superseded: {}", entry_id.0)
            }
        }
    }
}

impl std::error::Error for InstitutionalRecordError {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum InstitutionalBeliefKey {
    OfficeHolderOf {
        office: EntityId,
    },
    FactionMembersOf {
        faction: EntityId,
    },
    SupportFor {
        supporter: EntityId,
        office: EntityId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelievedInstitutionalClaim {
    pub claim: InstitutionalClaim,
    pub source: InstitutionalKnowledgeSource,
    pub learned_tick: Tick,
    pub learned_at: Option<EntityId>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum InstitutionalKnowledgeSource {
    WitnessedEvent,
    Report {
        from: EntityId,
        chain_len: u8,
    },
    RecordConsultation {
        record: EntityId,
        entry_id: RecordEntryId,
    },
    SelfDeclaration,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum InstitutionalBeliefRead<T> {
    Unknown,
    Certain(T),
    Conflicted(Vec<T>),
}

#[cfg(test)]
mod tests {
    use super::{
        BelievedInstitutionalClaim, InstitutionalBeliefKey, InstitutionalBeliefRead,
        InstitutionalClaim, InstitutionalKnowledgeSource, InstitutionalRecordEntry,
        InstitutionalRecordError, RecordData, RecordEntryId, RecordKind,
    };
    use crate::{traits::Component, EntityId, Tick};
    use serde::{de::DeserializeOwned, Serialize};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn office_holder_claim(holder: Option<EntityId>, effective_tick: u64) -> InstitutionalClaim {
        InstitutionalClaim::OfficeHolder {
            office: entity(10),
            holder,
            effective_tick: Tick(effective_tick),
        }
    }

    fn support_claim(candidate: Option<EntityId>, effective_tick: u64) -> InstitutionalClaim {
        InstitutionalClaim::SupportDeclaration {
            office: entity(10),
            supporter: entity(30),
            candidate,
            effective_tick: Tick(effective_tick),
        }
    }

    fn sample_record() -> RecordData {
        RecordData {
            record_kind: RecordKind::OfficeRegister,
            home_place: entity(1),
            issuer: entity(2),
            consultation_ticks: 4,
            max_entries_per_consult: 6,
            entries: Vec::new(),
            next_entry_id: 0,
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn institutional_types_satisfy_component_and_serde_bounds() {
        assert_component_bounds::<RecordData>();
        assert_value_bounds::<RecordData>();
        assert_value_bounds::<RecordKind>();
        assert_value_bounds::<RecordEntryId>();
        assert_value_bounds::<InstitutionalClaim>();
        assert_value_bounds::<InstitutionalRecordEntry>();
        assert_value_bounds::<InstitutionalBeliefKey>();
        assert_value_bounds::<BelievedInstitutionalClaim>();
        assert_value_bounds::<InstitutionalKnowledgeSource>();
        assert_value_bounds::<InstitutionalBeliefRead<Option<EntityId>>>();
    }

    #[test]
    fn record_data_append_entry_assigns_monotonic_entry_ids() {
        let mut record = sample_record();

        let first = record.append_entry(office_holder_claim(Some(entity(20)), 5), Tick(6));
        let second = record.append_entry(office_holder_claim(None, 7), Tick(8));

        assert_eq!(first, RecordEntryId(0));
        assert_eq!(second, RecordEntryId(1));
        assert_eq!(record.next_entry_id, 2);
        assert_eq!(record.entries.len(), 2);
        assert_eq!(record.entries[0].supersedes, None);
        assert_eq!(record.entries[1].supersedes, None);
    }

    #[test]
    fn record_data_supersede_entry_appends_successor() {
        let mut record = sample_record();
        let first = record.append_entry(office_holder_claim(Some(entity(20)), 5), Tick(6));

        let second = record
            .supersede_entry(first, office_holder_claim(None, 9), Tick(10))
            .unwrap();

        assert_eq!(second, RecordEntryId(1));
        assert_eq!(record.next_entry_id, 2);
        assert_eq!(record.entries[1].supersedes, Some(first));
        assert_eq!(record.entries[1].recorded_tick, Tick(10));
    }

    #[test]
    fn record_data_supersede_entry_rejects_missing_entry() {
        let mut record = sample_record();

        let err = record
            .supersede_entry(RecordEntryId(42), office_holder_claim(None, 9), Tick(10))
            .unwrap_err();

        assert_eq!(
            err,
            InstitutionalRecordError::EntryNotFound(RecordEntryId(42))
        );
    }

    #[test]
    fn record_data_supersede_entry_rejects_duplicate_supersession() {
        let mut record = sample_record();
        let first = record.append_entry(office_holder_claim(Some(entity(20)), 5), Tick(6));
        record
            .supersede_entry(first, office_holder_claim(None, 9), Tick(10))
            .unwrap();

        let err = record
            .supersede_entry(first, office_holder_claim(Some(entity(21)), 11), Tick(12))
            .unwrap_err();

        assert_eq!(err, InstitutionalRecordError::EntryAlreadySuperseded(first));
    }

    #[test]
    fn record_data_active_entries_excludes_superseded_entries() {
        let mut record = sample_record();
        let first = record.append_entry(office_holder_claim(Some(entity(20)), 5), Tick(6));
        let second = record.append_entry(support_claim(Some(entity(21)), 7), Tick(8));
        let third = record
            .supersede_entry(first, office_holder_claim(None, 9), Tick(10))
            .unwrap();

        let active_ids = record
            .active_entries()
            .into_iter()
            .map(|entry| entry.entry_id)
            .collect::<Vec<_>>();

        assert_eq!(active_ids, vec![second, third]);
    }

    #[test]
    fn record_data_entries_newest_first_uses_append_order() {
        let mut record = sample_record();
        let first = record.append_entry(office_holder_claim(Some(entity(20)), 5), Tick(6));
        let second = record.append_entry(office_holder_claim(None, 4), Tick(7));
        let third = record.append_entry(office_holder_claim(Some(entity(21)), 3), Tick(8));

        let ordered_ids = record
            .entries_newest_first()
            .map(|entry| entry.entry_id)
            .collect::<Vec<_>>();

        assert_eq!(ordered_ids, vec![third, second, first]);
    }

    #[test]
    fn institutional_belief_key_ordering_is_deterministic() {
        let mut keys = vec![
            InstitutionalBeliefKey::SupportFor {
                supporter: entity(4),
                office: entity(3),
            },
            InstitutionalBeliefKey::OfficeHolderOf { office: entity(7) },
            InstitutionalBeliefKey::FactionMembersOf { faction: entity(2) },
        ];

        keys.sort();

        assert_eq!(
            keys,
            vec![
                InstitutionalBeliefKey::OfficeHolderOf { office: entity(7) },
                InstitutionalBeliefKey::FactionMembersOf { faction: entity(2) },
                InstitutionalBeliefKey::SupportFor {
                    supporter: entity(4),
                    office: entity(3),
                },
            ]
        );
    }

    #[test]
    fn institutional_types_roundtrip_through_bincode() {
        let mut record = sample_record();
        let entry_id = record.append_entry(office_holder_claim(None, 9), Tick(10));

        let belief = BelievedInstitutionalClaim {
            claim: support_claim(None, 11),
            source: InstitutionalKnowledgeSource::RecordConsultation {
                record: entity(40),
                entry_id,
            },
            learned_tick: Tick(12),
            learned_at: Some(entity(1)),
        };
        let read = InstitutionalBeliefRead::Conflicted(vec![
            support_claim(Some(entity(41)), 13),
            support_claim(None, 14),
        ]);

        let record_bytes = bincode::serialize(&record).unwrap();
        let record_roundtrip: RecordData = bincode::deserialize(&record_bytes).unwrap();
        assert_eq!(record_roundtrip, record);

        let belief_bytes = bincode::serialize(&belief).unwrap();
        let belief_roundtrip: BelievedInstitutionalClaim =
            bincode::deserialize(&belief_bytes).unwrap();
        assert_eq!(belief_roundtrip, belief);

        let read_bytes = bincode::serialize(&read).unwrap();
        let read_roundtrip: InstitutionalBeliefRead<InstitutionalClaim> =
            bincode::deserialize(&read_bytes).unwrap();
        assert_eq!(read_roundtrip, read);
    }

    #[test]
    fn nullable_vacancy_and_no_candidate_claims_are_preserved() {
        let office_claim = office_holder_claim(None, 15);
        let support_claim = support_claim(None, 16);

        match office_claim {
            InstitutionalClaim::OfficeHolder { holder, .. } => assert_eq!(holder, None),
            other => panic!("expected office holder claim, got {other:?}"),
        }

        match support_claim {
            InstitutionalClaim::SupportDeclaration { candidate, .. } => {
                assert_eq!(candidate, None)
            }
            other => panic!("expected support declaration claim, got {other:?}"),
        }
    }
}
