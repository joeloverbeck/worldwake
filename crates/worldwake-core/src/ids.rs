//! Core identity types for the Worldwake simulation.
//!
//! All types are `Copy + Clone + Eq + Ord + Hash + Debug + Display +
//! Serialize + Deserialize` — the minimum set required for deterministic
//! authoritative state.

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};

#[doc(hidden)]
#[macro_export]
macro_rules! worldwake_prefixed_id_type {
    ($(#[$meta:meta])* $vis:vis struct $name:ident($inner:ty, $prefix:literal);) => {
        $(#[$meta])*
        #[derive(
            Copy,
            Clone,
            Eq,
            PartialEq,
            Ord,
            PartialOrd,
            Hash,
            Debug,
            serde::Serialize,
            serde::Deserialize,
        )]
        $vis struct $name(pub $inner);

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, concat!($prefix, "{}"), self.0)
            }
        }
    };
}

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

crate::worldwake_prefixed_id_type!(
    /// Unique identifier for an event in the append-only log.
    pub struct EventId(u64, "ev");
);

crate::worldwake_prefixed_id_type!(
    /// Stable identifier for a registered action definition.
    pub struct ActionDefId(u32, "adef");
);

crate::worldwake_prefixed_id_type!(
    /// Unique identifier for a reservation record in the relation layer.
    pub struct ReservationId(u64, "r");
);

/// Half-open tick interval `[start, end)` used for reservation windows.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct TickRange {
    start: Tick,
    end: Tick,
}

impl TickRange {
    pub fn new(start: Tick, end: Tick) -> Result<Self, &'static str> {
        if end <= start {
            return Err("tick range end must be greater than start");
        }

        Ok(Self { start, end })
    }

    pub fn start(&self) -> Tick {
        self.start
    }

    pub fn end(&self) -> Tick {
        self.end
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn contains_tick(&self, tick: Tick) -> bool {
        self.start <= tick && tick < self.end
    }
}

impl<'de> Deserialize<'de> for TickRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct TickRangeRepr {
            start: Tick,
            end: Tick,
        }

        let repr = TickRangeRepr::deserialize(deserializer)?;
        TickRange::new(repr.start, repr.end).map_err(de::Error::custom)
    }
}

impl fmt::Display for TickRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{},{})", self.start, self.end)
    }
}

crate::worldwake_prefixed_id_type!(
    /// Unique identifier for a directed travel edge in the topology graph.
    pub struct TravelEdgeId(u32, "te");
);

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
        assert_bounds::<ActionDefId>();
        assert_bounds::<ReservationId>();
        assert_bounds::<TickRange>();
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
    fn action_def_id_display_and_bincode_roundtrip() {
        let val = ActionDefId(14);
        assert_eq!(val.to_string(), "adef14");

        let bytes = bincode::serialize(&val).unwrap();
        let back: ActionDefId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn reservation_id_display_and_bincode_roundtrip() {
        let val = ReservationId(14);
        assert_eq!(val.to_string(), "r14");

        let bytes = bincode::serialize(&val).unwrap();
        let back: ReservationId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn tick_range_new_rejects_empty_or_inverted_ranges() {
        assert_eq!(
            TickRange::new(Tick(5), Tick(10)).unwrap().to_string(),
            "[t5,t10)"
        );
        assert!(TickRange::new(Tick(5), Tick(5)).is_err());
        assert!(TickRange::new(Tick(10), Tick(5)).is_err());
    }

    #[test]
    fn tick_range_overlap_uses_half_open_semantics() {
        let left = TickRange::new(Tick(3), Tick(7)).unwrap();
        let right = TickRange::new(Tick(5), Tick(10)).unwrap();
        let adjacent_left = TickRange::new(Tick(3), Tick(5)).unwrap();
        let adjacent_right = TickRange::new(Tick(5), Tick(10)).unwrap();

        assert!(left.overlaps(&right));
        assert!(right.overlaps(&left));
        assert!(!adjacent_left.overlaps(&adjacent_right));
        assert!(!adjacent_right.overlaps(&adjacent_left));
    }

    #[test]
    fn tick_range_contains_start_but_not_end() {
        let range = TickRange::new(Tick(5), Tick(10)).unwrap();

        assert!(range.contains_tick(Tick(5)));
        assert!(range.contains_tick(Tick(9)));
        assert!(!range.contains_tick(Tick(10)));
        assert!(!range.contains_tick(Tick(4)));
    }

    #[test]
    fn tick_range_bincode_roundtrip() {
        let val = TickRange::new(Tick(8), Tick(13)).unwrap();
        let bytes = bincode::serialize(&val).unwrap();
        let back: TickRange = bincode::deserialize(&bytes).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn tick_range_accessors_expose_bounds() {
        let range = TickRange::new(Tick(8), Tick(13)).unwrap();

        assert_eq!(range.start(), Tick(8));
        assert_eq!(range.end(), Tick(13));
    }

    #[test]
    fn tick_range_deserialization_rejects_empty_or_inverted_ranges() {
        let empty = bincode::serialize(&(Tick(5), Tick(5))).unwrap();
        let inverted = bincode::serialize(&(Tick(10), Tick(5))).unwrap();

        let empty_err = bincode::deserialize::<TickRange>(&empty)
            .unwrap_err()
            .to_string();
        let inverted_err = bincode::deserialize::<TickRange>(&inverted)
            .unwrap_err()
            .to_string();

        assert!(
            empty_err.contains("tick range end must be greater than start"),
            "unexpected empty-range error: {empty_err}"
        );
        assert!(
            inverted_err.contains("tick range end must be greater than start"),
            "unexpected inverted-range error: {inverted_err}"
        );
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
