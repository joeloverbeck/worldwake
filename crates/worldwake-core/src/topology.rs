//! Deterministic topology primitives for the world place graph.

use crate::{Component, EntityId, Permille, TravelEdgeId, WorldError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::num::{NonZeroU16, NonZeroU32};

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

/// Directed connection between two places in the topology graph.
#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct TravelEdge {
    id: TravelEdgeId,
    from: EntityId,
    to: EntityId,
    travel_time_ticks: NonZeroU32,
    capacity: Option<NonZeroU16>,
    danger: Permille,
    visibility: Permille,
}

impl TravelEdge {
    pub fn new(
        id: TravelEdgeId,
        from: EntityId,
        to: EntityId,
        travel_time_ticks: u32,
        capacity: Option<NonZeroU16>,
        danger: Permille,
        visibility: Permille,
    ) -> Result<Self, WorldError> {
        let travel_time_ticks = NonZeroU32::new(travel_time_ticks).ok_or_else(|| {
            WorldError::InvariantViolation("travel edge travel_time_ticks must be >= 1".into())
        })?;

        Ok(Self {
            id,
            from,
            to,
            travel_time_ticks,
            capacity,
            danger,
            visibility,
        })
    }

    pub fn id(&self) -> TravelEdgeId {
        self.id
    }

    pub fn from(&self) -> EntityId {
        self.from
    }

    pub fn to(&self) -> EntityId {
        self.to
    }

    pub fn travel_time_ticks(&self) -> u32 {
        self.travel_time_ticks.get()
    }

    pub fn capacity(&self) -> Option<NonZeroU16> {
        self.capacity
    }

    pub fn danger(&self) -> Permille {
        self.danger
    }

    pub fn visibility(&self) -> Permille {
        self.visibility
    }
}

#[cfg(test)]
mod tests {
    use super::{Place, PlaceTag, TravelEdge};
    use crate::test_utils::canonical_bytes;
    use crate::{traits::Component, EntityId, Permille, TravelEdgeId, WorldError};
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};
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

    #[test]
    fn travel_edge_construction_rejects_zero_ticks() {
        let err = TravelEdge::new(
            TravelEdgeId(1),
            EntityId {
                slot: 1,
                generation: 0,
            },
            EntityId {
                slot: 2,
                generation: 0,
            },
            0,
            None,
            Permille::new(50).unwrap(),
            Permille::new(900).unwrap(),
        )
        .unwrap_err();

        assert!(matches!(err, WorldError::InvariantViolation(_)));
        assert_eq!(
            err.to_string(),
            "invariant violation: travel edge travel_time_ticks must be >= 1"
        );
    }

    #[test]
    fn travel_edge_construction_accepts_minimum_valid_ticks() {
        let edge = TravelEdge::new(
            TravelEdgeId(7),
            EntityId {
                slot: 3,
                generation: 0,
            },
            EntityId {
                slot: 4,
                generation: 0,
            },
            1,
            NonZeroU16::new(6),
            Permille::new(125).unwrap(),
            Permille::new(875).unwrap(),
        )
        .unwrap();

        assert_eq!(edge.id(), TravelEdgeId(7));
        assert_eq!(
            edge.from(),
            EntityId {
                slot: 3,
                generation: 0,
            }
        );
        assert_eq!(
            edge.to(),
            EntityId {
                slot: 4,
                generation: 0,
            }
        );
        assert_eq!(edge.travel_time_ticks(), 1);
        assert_eq!(edge.capacity(), NonZeroU16::new(6));
        assert_eq!(edge.danger(), Permille::new(125).unwrap());
        assert_eq!(edge.visibility(), Permille::new(875).unwrap());
    }

    #[test]
    fn travel_edge_roundtrips_with_permille_fields() {
        let edge = TravelEdge::new(
            TravelEdgeId(11),
            EntityId {
                slot: 5,
                generation: 1,
            },
            EntityId {
                slot: 9,
                generation: 0,
            },
            12,
            NonZeroU16::new(3),
            Permille::new(0).unwrap(),
            Permille::new(1000).unwrap(),
        )
        .unwrap();

        let bytes = bincode::serialize(&edge).unwrap();
        let roundtrip: TravelEdge = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, edge);
        assert_eq!(roundtrip.travel_time_ticks(), 12);
        assert_eq!(roundtrip.danger(), Permille::new(0).unwrap());
        assert_eq!(roundtrip.visibility(), Permille::new(1000).unwrap());
    }

    #[derive(Serialize, Deserialize)]
    struct RawTravelEdge {
        id: TravelEdgeId,
        from: EntityId,
        to: EntityId,
        travel_time_ticks: u32,
        capacity: Option<NonZeroU16>,
        danger: Permille,
        visibility: Permille,
    }

    #[test]
    fn travel_edge_deserialization_rejects_zero_ticks() {
        let bytes = bincode::serialize(&RawTravelEdge {
            id: TravelEdgeId(99),
            from: EntityId {
                slot: 1,
                generation: 0,
            },
            to: EntityId {
                slot: 2,
                generation: 0,
            },
            travel_time_ticks: 0,
            capacity: None,
            danger: Permille::new(200).unwrap(),
            visibility: Permille::new(800).unwrap(),
        })
        .unwrap();

        let err = bincode::deserialize::<TravelEdge>(&bytes).unwrap_err();
        assert!(
            err.to_string().contains("invalid value: integer `0`"),
            "unexpected error: {err}"
        );
    }
}
