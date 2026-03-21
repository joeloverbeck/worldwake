use crate::{EventLog, World};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalError {
    message: String,
}

impl CanonicalError {
    fn serialization(message: String) -> Self {
        Self { message }
    }
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "canonical serialization error: {}", self.message)
    }
}

impl std::error::Error for CanonicalError {}

impl From<Box<bincode::ErrorKind>> for CanonicalError {
    fn from(value: Box<bincode::ErrorKind>) -> Self {
        Self::serialization(value.to_string())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct StateHash(pub [u8; 32]);

impl fmt::Display for StateHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

pub fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonicalError> {
    bincode::serialize(value).map_err(CanonicalError::from)
}

#[must_use]
pub fn hash_bytes(bytes: &[u8]) -> StateHash {
    StateHash(*blake3::hash(bytes).as_bytes())
}

pub fn hash_serializable<T: Serialize>(value: &T) -> Result<StateHash, CanonicalError> {
    canonical_bytes(value).map(|bytes| hash_bytes(&bytes))
}

pub fn hash_world(world: &World) -> Result<StateHash, CanonicalError> {
    hash_serializable(world)
}

pub fn hash_event_log(event_log: &EventLog) -> Result<StateHash, CanonicalError> {
    hash_serializable(event_log)
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_bytes, hash_bytes, hash_event_log, hash_serializable, hash_world, StateHash,
    };
    use crate::{
        build_prototype_world, CauseRef, EventLog, EventTag, InstitutionalClaim,
        InstitutionalRecordEntry, PrototypePlace, RecordData, RecordEntryId, RecordKind,
        VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt;

    fn assert_traits<T>()
    where
        T: Copy
            + Clone
            + Eq
            + Ord
            + std::hash::Hash
            + std::fmt::Debug
            + fmt::Display
            + Serialize
            + DeserializeOwned,
    {
    }

    fn populated_world() -> World {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut txn = WorldTxn::new(
            &mut world,
            crate::Tick(0),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        let _ = txn
            .create_agent("hash-agent", crate::ControlSource::Ai)
            .unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
        world
    }

    fn populated_event_log() -> EventLog {
        let mut log = EventLog::new();
        let _ = log.emit(crate::PendingEvent::from_payload(crate::EventPayload {
            tick: crate::Tick(0),
            cause: CauseRef::Bootstrap,
            actor_id: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: None,
            state_deltas: Vec::new(),
            observed_entities: std::collections::BTreeMap::new(),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags: std::collections::BTreeSet::from([EventTag::System]),
        }));
        log
    }

    #[test]
    fn state_hash_satisfies_required_traits() {
        assert_traits::<StateHash>();
    }

    #[test]
    fn state_hash_display_is_lowercase_hex() {
        let hash = StateHash([0xab; 32]);

        assert_eq!(hash.to_string(), "ab".repeat(32));
    }

    #[test]
    fn canonical_bytes_are_stable_for_identical_values() {
        let left = canonical_bytes(&("same", 7u32)).unwrap();
        let right = canonical_bytes(&("same", 7u32)).unwrap();

        assert_eq!(left, right);
    }

    #[test]
    fn hash_bytes_matches_direct_blake3_output() {
        let bytes = b"worldwake";

        assert_eq!(
            hash_bytes(bytes),
            StateHash(*blake3::hash(bytes).as_bytes())
        );
    }

    #[test]
    fn hash_serializable_is_stable_for_identical_values() {
        let left = hash_serializable(&vec![1u32, 2, 3]).unwrap();
        let right = hash_serializable(&vec![1u32, 2, 3]).unwrap();

        assert_eq!(left, right);
    }

    #[test]
    fn hash_serializable_changes_when_value_changes() {
        let left = hash_serializable(&vec![1u32, 2, 3]).unwrap();
        let right = hash_serializable(&vec![1u32, 2, 4]).unwrap();

        assert_ne!(left, right);
    }

    #[test]
    fn hash_world_is_stable_for_identical_worlds() {
        let left = populated_world();
        let right = left.clone();

        assert_eq!(hash_world(&left).unwrap(), hash_world(&right).unwrap());
    }

    #[test]
    fn hash_world_changes_when_world_changes() {
        let original = populated_world();
        let mut changed = original.clone();
        let mut txn = WorldTxn::new(
            &mut changed,
            crate::Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        let _ = txn
            .create_agent("second-agent", crate::ControlSource::Ai)
            .unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);

        assert_ne!(
            hash_world(&original).unwrap(),
            hash_world(&changed).unwrap()
        );
    }

    #[test]
    fn hash_world_changes_when_record_data_changes() {
        let mut original = World::new(build_prototype_world()).unwrap();
        let record = RecordData {
            record_kind: RecordKind::OfficeRegister,
            home_place: crate::prototype_place_entity(PrototypePlace::VillageSquare),
            issuer: crate::prototype_place_entity(PrototypePlace::VillageSquare),
            consultation_ticks: 4,
            max_entries_per_consult: 6,
            entries: vec![InstitutionalRecordEntry {
                entry_id: RecordEntryId(0),
                claim: InstitutionalClaim::OfficeHolder {
                    office: crate::EntityId {
                        slot: 1000,
                        generation: 0,
                    },
                    holder: None,
                    effective_tick: crate::Tick(2),
                },
                recorded_tick: crate::Tick(3),
                supersedes: None,
            }],
            next_entry_id: 1,
        };
        let record_id = original.create_record(record.clone(), crate::Tick(1)).unwrap();
        let mut changed = original.clone();
        let mut updated = record;
        updated.max_entries_per_consult = 7;
        changed.remove_component_record_data(record_id).unwrap();
        changed
            .insert_component_record_data(record_id, updated)
            .unwrap();

        assert_ne!(
            hash_world(&original).unwrap(),
            hash_world(&changed).unwrap()
        );
    }

    #[test]
    fn hash_event_log_is_stable_for_identical_logs() {
        let left = populated_event_log();
        let right = left.clone();

        assert_eq!(
            hash_event_log(&left).unwrap(),
            hash_event_log(&right).unwrap()
        );
    }

    #[test]
    fn hash_event_log_changes_when_event_is_appended() {
        let original = populated_event_log();
        let mut changed = original.clone();
        let _ = changed.emit(crate::PendingEvent::from_payload(crate::EventPayload {
            tick: crate::Tick(1),
            cause: CauseRef::SystemTick(crate::Tick(1)),
            actor_id: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: None,
            state_deltas: Vec::new(),
            observed_entities: std::collections::BTreeMap::new(),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags: std::collections::BTreeSet::from([EventTag::System]),
        }));

        assert_ne!(
            hash_event_log(&original).unwrap(),
            hash_event_log(&changed).unwrap()
        );
    }

    #[test]
    fn state_hash_roundtrips_through_bincode() {
        let hash = StateHash([42; 32]);

        let bytes = bincode::serialize(&hash).unwrap();
        let roundtrip: StateHash = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, hash);
    }
}
