//! Deterministic topology primitives for the world place graph.

use crate::{Component, EntityId, Permille, TravelEdgeId, WorldError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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

/// Ordered storage for the world place graph and deterministic query APIs.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Topology {
    places: BTreeMap<EntityId, Place>,
    edges: BTreeMap<TravelEdgeId, TravelEdge>,
    outgoing: BTreeMap<EntityId, Vec<TravelEdgeId>>,
    incoming: BTreeMap<EntityId, Vec<TravelEdgeId>>,
}

impl Topology {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_place(&mut self, id: EntityId, place: Place) -> Result<(), WorldError> {
        if self.places.contains_key(&id) {
            return Err(WorldError::InvalidOperation(format!(
                "duplicate place id: {id}"
            )));
        }

        self.places.insert(id, place);
        Ok(())
    }

    pub fn add_edge(&mut self, edge: TravelEdge) -> Result<(), WorldError> {
        if self.edges.contains_key(&edge.id()) {
            return Err(WorldError::InvalidOperation(format!(
                "duplicate travel edge id: {}",
                edge.id()
            )));
        }
        if !self.places.contains_key(&edge.from()) {
            return Err(WorldError::EntityNotFound(edge.from()));
        }
        if !self.places.contains_key(&edge.to()) {
            return Err(WorldError::EntityNotFound(edge.to()));
        }

        let edge_id = edge.id();
        let from = edge.from();
        let to = edge.to();
        self.edges.insert(edge_id, edge);
        insert_sorted_edge_id(self.outgoing.entry(from).or_default(), edge_id);
        insert_sorted_edge_id(self.incoming.entry(to).or_default(), edge_id);
        Ok(())
    }

    pub fn place(&self, id: EntityId) -> Option<&Place> {
        self.places.get(&id)
    }

    pub fn edge(&self, id: TravelEdgeId) -> Option<&TravelEdge> {
        self.edges.get(&id)
    }

    pub fn outgoing_edges(&self, place: EntityId) -> &[TravelEdgeId] {
        self.outgoing.get(&place).map_or(&[], Vec::as_slice)
    }

    pub fn incoming_edges(&self, place: EntityId) -> &[TravelEdgeId] {
        self.incoming.get(&place).map_or(&[], Vec::as_slice)
    }

    pub fn neighbors(&self, place: EntityId) -> Vec<EntityId> {
        self.outgoing_edges(place)
            .iter()
            .filter_map(|edge_id| self.edge(*edge_id))
            .map(TravelEdge::to)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn is_reachable(&self, from: EntityId, to: EntityId) -> bool {
        if !self.places.contains_key(&from) || !self.places.contains_key(&to) {
            return false;
        }
        if from == to {
            return true;
        }

        let mut visited = BTreeSet::new();
        let mut frontier = vec![from];

        while let Some(current) = frontier.pop() {
            if !visited.insert(current) {
                continue;
            }

            let mut neighbors = self.neighbors(current);
            neighbors.reverse();
            for neighbor in neighbors {
                if neighbor == to {
                    return true;
                }
                if !visited.contains(&neighbor) {
                    frontier.push(neighbor);
                }
            }
        }

        false
    }

    pub fn place_count(&self) -> usize {
        self.places.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

fn insert_sorted_edge_id(edge_ids: &mut Vec<TravelEdgeId>, edge_id: TravelEdgeId) {
    match edge_ids.binary_search(&edge_id) {
        Ok(_) => {}
        Err(index) => edge_ids.insert(index, edge_id),
    }
}

#[cfg(test)]
mod tests {
    use super::{Place, PlaceTag, Topology, TravelEdge};
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

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn place(name: &str, tags: &[PlaceTag]) -> Place {
        Place {
            name: name.to_string(),
            capacity: None,
            tags: tags.iter().copied().collect(),
        }
    }

    fn edge(id: u32, from: u32, to: u32) -> TravelEdge {
        TravelEdge::new(
            TravelEdgeId(id),
            entity(from),
            entity(to),
            1,
            None,
            Permille::new(0).unwrap(),
            Permille::new(1000).unwrap(),
        )
        .unwrap()
    }

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

    #[test]
    fn topology_add_place_returns_inserted_place() {
        let mut topology = Topology::new();
        let square = place("Square", &[PlaceTag::Village, PlaceTag::Hall]);

        topology.add_place(entity(1), square.clone()).unwrap();

        assert_eq!(topology.place(entity(1)), Some(&square));
        assert_eq!(topology.place_count(), 1);
    }

    #[test]
    fn topology_add_place_rejects_duplicate_place_ids() {
        let mut topology = Topology::new();
        topology
            .add_place(entity(1), place("Square", &[PlaceTag::Village]))
            .unwrap();

        let err = topology
            .add_place(entity(1), place("Duplicate", &[PlaceTag::Hall]))
            .unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
        assert_eq!(err.to_string(), "invalid operation: duplicate place id: e1g0");
        assert_eq!(topology.place_count(), 1);
        assert_eq!(topology.place(entity(1)).unwrap().name, "Square");
    }

    #[test]
    fn topology_add_edge_returns_inserted_edge_and_sorted_adjacency() {
        let mut topology = Topology::new();
        topology
            .add_place(entity(1), place("A", &[PlaceTag::Village]))
            .unwrap();
        topology
            .add_place(entity(2), place("B", &[PlaceTag::Farm]))
            .unwrap();
        topology
            .add_place(entity(3), place("C", &[PlaceTag::Store]))
            .unwrap();

        topology.add_edge(edge(30, 1, 3)).unwrap();
        topology.add_edge(edge(10, 1, 2)).unwrap();
        topology.add_edge(edge(20, 3, 2)).unwrap();

        assert_eq!(topology.edge(TravelEdgeId(10)).unwrap().to(), entity(2));
        assert_eq!(topology.edge_count(), 3);
        assert_eq!(
            topology.outgoing_edges(entity(1)),
            &[TravelEdgeId(10), TravelEdgeId(30)]
        );
        assert_eq!(
            topology.incoming_edges(entity(2)),
            &[TravelEdgeId(10), TravelEdgeId(20)]
        );
    }

    #[test]
    fn topology_add_edge_rejects_duplicate_edge_ids() {
        let mut topology = Topology::new();
        topology
            .add_place(entity(1), place("A", &[PlaceTag::Village]))
            .unwrap();
        topology
            .add_place(entity(2), place("B", &[PlaceTag::Farm]))
            .unwrap();
        topology.add_edge(edge(7, 1, 2)).unwrap();

        let err = topology.add_edge(edge(7, 2, 1)).unwrap_err();

        assert!(matches!(err, WorldError::InvalidOperation(_)));
        assert_eq!(
            err.to_string(),
            "invalid operation: duplicate travel edge id: te7"
        );
        assert_eq!(topology.edge_count(), 1);
    }

    #[test]
    fn topology_add_edge_rejects_missing_endpoints() {
        let mut topology = Topology::new();
        topology
            .add_place(entity(1), place("A", &[PlaceTag::Village]))
            .unwrap();

        let missing_to = topology.add_edge(edge(1, 1, 2)).unwrap_err();
        assert!(matches!(missing_to, WorldError::EntityNotFound(id) if id == entity(2)));

        let missing_from = topology.add_edge(edge(2, 3, 1)).unwrap_err();
        assert!(matches!(missing_from, WorldError::EntityNotFound(id) if id == entity(3)));
        assert_eq!(topology.edge_count(), 0);
    }

    #[test]
    fn topology_neighbors_are_sorted_and_deduplicated() {
        let mut topology = Topology::new();
        for (slot, name, tag) in [
            (1, "A", PlaceTag::Village),
            (2, "B", PlaceTag::Farm),
            (3, "C", PlaceTag::Store),
        ] {
            topology.add_place(entity(slot), place(name, &[tag])).unwrap();
        }

        topology.add_edge(edge(30, 1, 3)).unwrap();
        topology.add_edge(edge(10, 1, 2)).unwrap();
        topology.add_edge(edge(20, 1, 2)).unwrap();

        assert_eq!(topology.neighbors(entity(1)), vec![entity(2), entity(3)]);
    }

    #[test]
    fn topology_reachability_matches_connected_and_disconnected_graphs() {
        let mut topology = Topology::new();
        for (slot, name, tag) in [
            (1, "A", PlaceTag::Village),
            (2, "B", PlaceTag::Farm),
            (3, "C", PlaceTag::Store),
            (4, "D", PlaceTag::Forest),
            (5, "E", PlaceTag::Camp),
        ] {
            topology.add_place(entity(slot), place(name, &[tag])).unwrap();
        }

        topology.add_edge(edge(10, 1, 2)).unwrap();
        topology.add_edge(edge(20, 2, 3)).unwrap();
        topology.add_edge(edge(30, 3, 4)).unwrap();

        assert!(topology.is_reachable(entity(1), entity(4)));
        assert!(topology.is_reachable(entity(3), entity(3)));
        assert!(!topology.is_reachable(entity(4), entity(1)));
        assert!(!topology.is_reachable(entity(1), entity(5)));
        assert!(!topology.is_reachable(entity(1), entity(99)));
    }

    #[test]
    fn topology_empty_queries_are_graceful() {
        let topology = Topology::new();

        assert_eq!(topology.place(entity(1)), None);
        assert_eq!(topology.edge(TravelEdgeId(1)), None);
        assert!(topology.outgoing_edges(entity(1)).is_empty());
        assert!(topology.incoming_edges(entity(1)).is_empty());
        assert!(topology.neighbors(entity(1)).is_empty());
        assert!(!topology.is_reachable(entity(1), entity(2)));
        assert_eq!(topology.place_count(), 0);
        assert_eq!(topology.edge_count(), 0);
    }
}
