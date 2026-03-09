//! Core identity types for the Worldwake simulation.
//!
//! All types are `Copy + Clone + Eq + Ord + Hash + Debug + Display +
//! Serialize + Deserialize` — the minimum set required for deterministic
//! authoritative state.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};

/// Stable entity identifier with generational slot reuse detection.
///
/// `slot` identifies the allocator slot; `generation` is bumped on
/// archival + reuse so stale references compare unequal.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct EntityId {
    pub slot: u32,
    pub generation: u32,
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}g{}", self.slot, self.generation)
    }
}

/// Discrete simulation tick (logical time).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct Tick(pub u64);

impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

impl Add<u64> for Tick {
    type Output = Self;
    fn add(self, rhs: u64) -> Self {
        Self(self.0 + rhs)
    }
}

impl Sub<u64> for Tick {
    type Output = Self;
    fn sub(self, rhs: u64) -> Self {
        Self(self.0 - rhs)
    }
}

/// Unique identifier for an event in the append-only log.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct EventId(pub u64);

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ev{}", self.0)
    }
}

/// Unique identifier for a directed travel edge in the topology graph.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct TravelEdgeId(pub u32);

impl fmt::Display for TravelEdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "te{}", self.0)
    }
}

/// Deterministic seed for `ChaCha8Rng`.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct Seed(pub [u8; 32]);

impl fmt::Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "seed[")?;
        for (i, byte) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ":")?;
            }
            write!(f, "{byte:02x}")?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- EntityId ---

    #[test]
    fn entity_id_stale_reference_detection() {
        let fresh = EntityId {
            slot: 0,
            generation: 1,
        };
        let stale = EntityId {
            slot: 0,
            generation: 0,
        };
        assert_ne!(
            fresh, stale,
            "same slot, different generation must be unequal"
        );
    }

    #[test]
    fn entity_id_deterministic_ordering() {
        let a = EntityId {
            slot: 0,
            generation: 5,
        };
        let b = EntityId {
            slot: 1,
            generation: 0,
        };
        assert!(a < b, "slot-major ordering");

        let c = EntityId {
            slot: 1,
            generation: 0,
        };
        let d = EntityId {
            slot: 1,
            generation: 1,
        };
        assert!(c < d, "generation-minor ordering");
    }

    #[test]
    fn entity_id_display() {
        let id = EntityId {
            slot: 42,
            generation: 3,
        };
        assert_eq!(id.to_string(), "e42g3");
    }

    // --- Tick ---

    #[test]
    fn tick_arithmetic() {
        assert_eq!(Tick(5) + 3, Tick(8));
        assert_eq!(Tick(8) - 3, Tick(5));
    }

    #[test]
    fn tick_ordering() {
        assert!(Tick(1) < Tick(2));
    }

    // --- Compile-time trait bound assertions ---

    fn assert_bounds<
        T: Copy
            + Clone
            + Eq
            + Ord
            + std::hash::Hash
            + std::fmt::Debug
            + std::fmt::Display
            + Serialize
            + serde::de::DeserializeOwned,
    >() {
    }

    #[test]
    fn id_types_satisfy_required_traits() {
        assert_bounds::<EntityId>();
        assert_bounds::<Tick>();
        assert_bounds::<EventId>();
        assert_bounds::<TravelEdgeId>();
        assert_bounds::<Seed>();
    }

    // --- Bincode round-trip ---

    #[test]
    fn entity_id_bincode_roundtrip() {
        let val = EntityId {
            slot: 99,
            generation: 7,
        };
        let bytes = bincode::serialize(&val).unwrap();
        let back: EntityId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn tick_bincode_roundtrip() {
        let val = Tick(12345);
        let bytes = bincode::serialize(&val).unwrap();
        let back: Tick = bincode::deserialize(&bytes).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn event_id_bincode_roundtrip() {
        let val = EventId(999);
        let bytes = bincode::serialize(&val).unwrap();
        let back: EventId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn travel_edge_id_display() {
        let id = TravelEdgeId(42);
        assert_eq!(id.to_string(), "te42");
    }

    #[test]
    fn travel_edge_id_bincode_roundtrip() {
        let val = TravelEdgeId(77);
        let bytes = bincode::serialize(&val).unwrap();
        let back: TravelEdgeId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn seed_bincode_roundtrip() {
        let val = Seed([42; 32]);
        let bytes = bincode::serialize(&val).unwrap();
        let back: Seed = bincode::deserialize(&bytes).unwrap();
        assert_eq!(val, back);
    }
}
