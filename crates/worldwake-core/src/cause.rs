//! Explicit causal references for event-log records.

use crate::{EventId, Tick};
use serde::{Deserialize, Serialize};

/// Direct cause of an event record.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum CauseRef {
    /// Caused by an earlier event in the append-only log.
    Event(EventId),
    /// Caused by a system-level tick progression.
    SystemTick(Tick),
    /// Caused during world bootstrap with no earlier event.
    Bootstrap,
    /// Caused by stable external input outside the simulation loop.
    ExternalInput(u64),
}

#[cfg(test)]
mod tests {
    use super::CauseRef;
    use crate::{EventId, Tick};
    use serde::{de::DeserializeOwned, Serialize};

    fn assert_traits<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + std::fmt::Debug + Serialize + DeserializeOwned,
    >() {
    }

    #[test]
    fn cause_ref_satisfies_required_traits() {
        assert_traits::<CauseRef>();
    }

    #[test]
    fn cause_ref_variants_construct_and_pattern_match() {
        let event = CauseRef::Event(EventId(7));
        let tick = CauseRef::SystemTick(Tick(11));
        let bootstrap = CauseRef::Bootstrap;
        let input = CauseRef::ExternalInput(19);

        assert!(matches!(event, CauseRef::Event(EventId(7))));
        assert!(matches!(tick, CauseRef::SystemTick(Tick(11))));
        assert!(matches!(bootstrap, CauseRef::Bootstrap));
        assert!(matches!(input, CauseRef::ExternalInput(19)));
    }

    #[test]
    fn cause_ref_root_causes_are_explicitly_distinct() {
        assert_ne!(CauseRef::Bootstrap, CauseRef::SystemTick(Tick(0)));
        assert_ne!(CauseRef::Bootstrap, CauseRef::ExternalInput(0));
        assert_ne!(CauseRef::SystemTick(Tick(0)), CauseRef::ExternalInput(0));
    }

    #[test]
    fn cause_ref_bincode_roundtrip_covers_every_variant() {
        let variants = [
            CauseRef::Event(EventId(3)),
            CauseRef::SystemTick(Tick(5)),
            CauseRef::Bootstrap,
            CauseRef::ExternalInput(8),
        ];

        for variant in variants {
            let bytes = bincode::serialize(&variant).unwrap();
            let roundtrip: CauseRef = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, variant);
        }
    }
}
