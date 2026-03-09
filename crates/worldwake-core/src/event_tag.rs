//! Stable event classification tags for indexing and query surfaces.

use serde::{Deserialize, Serialize};

/// Ordered event categories used by indices and filtering.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum EventTag {
    WorldMutation,
    Inventory,
    Transfer,
    Reservation,
    ActionStarted,
    ActionCommitted,
    ActionAborted,
    ActionInterrupted,
    Travel,
    Trade,
    Crime,
    Combat,
    Political,
    Control,
    System,
}

#[cfg(test)]
mod tests {
    use super::EventTag;
    use serde::{de::DeserializeOwned, Serialize};

    fn assert_traits<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + std::fmt::Debug + Serialize + DeserializeOwned,
    >() {
    }

    const ALL_EVENT_TAGS: [EventTag; 15] = [
        EventTag::WorldMutation,
        EventTag::Inventory,
        EventTag::Transfer,
        EventTag::Reservation,
        EventTag::ActionStarted,
        EventTag::ActionCommitted,
        EventTag::ActionAborted,
        EventTag::ActionInterrupted,
        EventTag::Travel,
        EventTag::Trade,
        EventTag::Crime,
        EventTag::Combat,
        EventTag::Political,
        EventTag::Control,
        EventTag::System,
    ];

    #[test]
    fn event_tag_satisfies_required_traits() {
        assert_traits::<EventTag>();
    }

    #[test]
    fn event_tag_includes_all_required_variants() {
        assert_eq!(ALL_EVENT_TAGS.len(), 15);
    }

    #[test]
    fn event_tag_order_is_declaration_stable() {
        let mut tags = ALL_EVENT_TAGS;
        tags.reverse();
        tags.sort_unstable();

        assert_eq!(tags, ALL_EVENT_TAGS);
    }

    #[test]
    fn event_tag_bincode_roundtrip_covers_every_variant() {
        for tag in ALL_EVENT_TAGS {
            let bytes = bincode::serialize(&tag).unwrap();
            let roundtrip: EventTag = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, tag);
        }
    }
}
