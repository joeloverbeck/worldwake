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
        build_prototype_world, CauseRef, EventLog, EventTag, VisibilitySpec, WitnessData, World,
        WorldTxn,
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
        let _ = log.emit(crate::PendingEvent::new(
            crate::Tick(0),
            CauseRef::Bootstrap,
            None,
            Vec::new(),
            None,
            Vec::new(),
            VisibilitySpec::Hidden,
            WitnessData::default(),
            std::collections::BTreeSet::from([EventTag::System]),
        ));
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
        let _ = changed.emit(crate::PendingEvent::new(
            crate::Tick(1),
            CauseRef::SystemTick(crate::Tick(1)),
            None,
            Vec::new(),
            None,
            Vec::new(),
            VisibilitySpec::Hidden,
            WitnessData::default(),
            std::collections::BTreeSet::from([EventTag::System]),
        ));

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
