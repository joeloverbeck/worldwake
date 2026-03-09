//! Entity classification and lifecycle metadata.

use crate::Tick;
use serde::{Deserialize, Serialize};

/// Classifies every entity for invariant checking and system routing.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum EntityKind {
    Agent,
    ItemLot,
    UniqueItem,
    Container,
    Facility,
    Place,
    Faction,
    Office,
    Contract,
    Rumor,
}

/// Authoritative metadata for a single entity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EntityMeta {
    pub kind: EntityKind,
    pub created_at: Tick,
    pub archived_at: Option<Tick>,
}

#[cfg(test)]
mod tests {
    use super::{EntityKind, EntityMeta};
    use crate::Tick;
    use serde::{de::DeserializeOwned, Serialize};

    const ALL_ENTITY_KINDS: [EntityKind; 10] = [
        EntityKind::Agent,
        EntityKind::ItemLot,
        EntityKind::UniqueItem,
        EntityKind::Container,
        EntityKind::Facility,
        EntityKind::Place,
        EntityKind::Faction,
        EntityKind::Office,
        EntityKind::Contract,
        EntityKind::Rumor,
    ];

    fn assert_entity_kind_bounds<
        T: Copy
            + Clone
            + Eq
            + Ord
            + std::hash::Hash
            + std::fmt::Debug
            + Serialize
            + DeserializeOwned,
    >() {
    }

    #[test]
    fn entity_kind_trait_bounds() {
        assert_entity_kind_bounds::<EntityKind>();
    }

    #[test]
    fn entity_kind_all_variants_bincode_roundtrip() {
        for kind in ALL_ENTITY_KINDS {
            let bytes = bincode::serialize(&kind).unwrap();
            let roundtrip: EntityKind = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, kind);
        }
    }

    #[test]
    fn entity_meta_bincode_roundtrip_alive() {
        let meta = EntityMeta {
            kind: EntityKind::Agent,
            created_at: Tick(7),
            archived_at: None,
        };

        let bytes = bincode::serialize(&meta).unwrap();
        let roundtrip: EntityMeta = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, meta);
    }

    #[test]
    fn entity_meta_bincode_roundtrip_archived() {
        let meta = EntityMeta {
            kind: EntityKind::Facility,
            created_at: Tick(7),
            archived_at: Some(Tick(42)),
        };

        let bytes = bincode::serialize(&meta).unwrap();
        let roundtrip: EntityMeta = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, meta);
    }

    #[test]
    fn entity_kind_deterministic_ordering() {
        let mut reversed = ALL_ENTITY_KINDS;
        reversed.reverse();
        reversed.sort();
        assert_eq!(reversed, ALL_ENTITY_KINDS);
    }
}
