//! Deterministic topology primitives for the world place graph.

use crate::Component;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::num::NonZeroU16;

/// Categorizes a place in the world graph.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum PlaceTag {
    Village,
    Farm,
    Store,
    Inn,
    Hall,
    Barracks,
    Latrine,
    Crossroads,
    Forest,
    Camp,
    Road,
    Trail,
    Field,
    Gate,
}

/// Authoritative metadata for a place entity in the world graph.
#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
    pub capacity: Option<NonZeroU16>,
    pub tags: BTreeSet<PlaceTag>,
}

impl Component for Place {}

#[cfg(test)]
mod tests {
    use super::{Place, PlaceTag};
    use crate::test_utils::canonical_bytes;
    use crate::traits::Component;
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use std::collections::BTreeSet;
    use std::num::NonZeroU16;

    fn assert_place_tag_traits<T>()
    where
        T: Copy
            + Clone
            + Eq
            + Ord
            + std::hash::Hash
            + std::fmt::Debug
            + Serialize
            + DeserializeOwned,
    {
    }

    fn assert_component_bounds<T: Component>() {}

    #[test]
    fn place_tag_satisfies_required_traits() {
        assert_place_tag_traits::<PlaceTag>();
    }

    #[test]
    fn place_satisfies_component_bounds() {
        assert_component_bounds::<Place>();
    }

    #[test]
    fn place_tag_btree_set_roundtrip_is_insertion_order_independent() {
        let place_a = Place {
            name: "Forest Road".to_string(),
            capacity: None,
            tags: BTreeSet::from([PlaceTag::Forest, PlaceTag::Road, PlaceTag::Camp]),
        };
        let place_b = Place {
            name: "Forest Road".to_string(),
            capacity: None,
            tags: BTreeSet::from([PlaceTag::Camp, PlaceTag::Forest, PlaceTag::Road]),
        };

        assert_eq!(place_a, place_b);
        assert_eq!(canonical_bytes(&place_a), canonical_bytes(&place_b));
    }

    #[test]
    fn place_roundtrips_with_absent_capacity() {
        let place = Place {
            name: "Crossroads".to_string(),
            capacity: None,
            tags: BTreeSet::from([PlaceTag::Crossroads, PlaceTag::Road]),
        };

        let bytes = bincode::serialize(&place).unwrap();
        let roundtrip: Place = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip.name, "Crossroads");
        assert_eq!(roundtrip.capacity, None);
        assert_eq!(roundtrip.tags, place.tags);
    }

    #[test]
    fn place_roundtrips_with_capacity() {
        let place = Place {
            name: "Village Square".to_string(),
            capacity: NonZeroU16::new(32),
            tags: BTreeSet::from([PlaceTag::Village, PlaceTag::Hall]),
        };

        let bytes = bincode::serialize(&place).unwrap();
        let roundtrip: Place = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip.name, place.name);
        assert_eq!(roundtrip.capacity, place.capacity);
        assert_eq!(roundtrip.tags, place.tags);
    }
}
