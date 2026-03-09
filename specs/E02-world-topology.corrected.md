# E02: Deterministic World Topology

## Epic Summary
Implement the directed graph world representation with places, travel edges, deterministic pathfinding, and a prototype world builder.

This epic is not just “have a graph.” It must define topology in a way that is safe for replay, stable hashing, and later movement / route danger logic.

## Phase
Phase 1: World Legality

## Crate
`worldwake-core`

## Dependencies
- E01 (deterministic core types and numeric wrappers)

## Why this revision exists
The original version used floating-point `danger` and `visibility` fields and did not define a deterministic tie-break rule for equal-cost routes. Both are avoidable risks in a replay-gated prototype.

Phase 1 topology should be:
- fixed-point, not float-based
- ordered and serializable
- deterministic even when multiple shortest paths exist

## Deliverables

### PlaceTag
- `PlaceTag` enum covering at minimum:
  - `Village`
  - `Farm`
  - `Store`
  - `Inn`
  - `Hall`
  - `Barracks`
  - `Latrine`
  - `Crossroads`
  - `Forest`
  - `Camp`
  - optional connective tags such as `Road`, `Trail`, `Field`, `Gate`

### Place Component
`Place` component / struct:
- `name: String`
- `capacity: Option<NonZeroU16>`
- `tags: BTreeSet<PlaceTag>`

Rules:
- place names in the prototype builder are stable and deterministic
- place tags are stored in sorted order
- place capacity is authoritative only if gameplay later uses it; otherwise it is metadata

### TravelEdgeId
Introduce a stable edge identifier:
- `TravelEdgeId(u32)`

Reason:
- deterministic path tie-breaks
- stable serialization
- future edge occupancy / travel event references

### TravelEdge
`TravelEdge` struct:
- `id: TravelEdgeId`
- `from: EntityId`
- `to: EntityId`
- `travel_time_ticks: u32`
- `capacity: Option<NonZeroU16>`
- `danger: Permille`
- `visibility: Permille`

Rules:
- `travel_time_ticks >= 1`
- `danger` and `visibility` are fixed-point values in `0..=1000`
- no floats in authoritative topology state

### Topology Storage
Use an ordered topology model:
- `places: BTreeMap<EntityId, Place>`
- `edges: BTreeMap<TravelEdgeId, TravelEdge>`
- `outgoing: BTreeMap<EntityId, Vec<TravelEdgeId>>`
- `incoming: BTreeMap<EntityId, Vec<TravelEdgeId>>`

Requirements:
- outgoing / incoming edge lists are stored in stable sorted order
- all topology queries return deterministic ordering

### Queries
Provide deterministic read APIs:
- `place(id) -> Option<&Place>`
- `edge(id) -> Option<&TravelEdge>`
- `outgoing_edges(place) -> &[TravelEdgeId]`
- `incoming_edges(place) -> &[TravelEdgeId]`
- `neighbors(place) -> Vec<EntityId>`
- `is_reachable(from, to) -> bool`

### Route Type
Return routes as a first-class value:
- `Route`
  - `places: Vec<EntityId>`
  - `edges: Vec<TravelEdgeId>`
  - `total_travel_time: u32`

Returning only a vector of place ids is too weak for later movement, occupancy, and event provenance.

### Pathfinding
Implement shortest path by `travel_time_ticks` using Dijkstra (A* is fine if the heuristic is documented and deterministic).

Determinism requirements:
- if multiple routes have equal total travel time, choose the one with the lexicographically smallest `edges` sequence
- internal priority queue ordering must not depend on hash iteration or pointer identity
- disconnected subgraphs return `None`

### Prototype World Builder
Build the prototype place graph from a fixed manifest order.

Required places per spec:
- 1 village core
- 1 farm / orchard
- 1 general store
- 1 inn or communal house
- 1 ruler's hall
- 1 barracks / guard post
- 1 latrine / toilet facility
- 1 crossroads
- 1 forest route
- 1 bandit camp
- 12-20 total place nodes

Requirements:
- the generated graph is navigable and intentionally connected for the playable micro-world
- each prototype place has at least one incoming and one outgoing edge
- builder output is stable: same code path => same place ids, edge ids, and topology hash

## Invariants Enforced
- Spec 3.3: world space is a place graph, not a continuous map
- Spec 5.2: topology uses directed edges with travel time, capacity, danger, visibility
- Spec 9.10: travel must use valid routes
- Spec 9.2: route queries are deterministic

## Tests
- [ ] Graph traversal visits all reachable nodes
- [ ] Prototype builder creates 12-20 places
- [ ] Every prototype place has at least one incoming and one outgoing edge
- [ ] Path costs equal the sum of edge travel times
- [ ] Pathfinding returns `None` for disconnected nodes in a test topology
- [ ] Equal-cost shortest paths resolve with the documented deterministic tie-break
- [ ] `TravelEdge` danger and visibility are always in `0..=1000`
- [ ] Topology serialization round-trips
- [ ] Prototype builder yields a stable topology hash across repeated runs in the same build

## Acceptance Criteria
- ordered graph storage with efficient neighbor lookups
- deterministic shortest-path queries
- no floats in authoritative topology state
- prototype builder produces a valid connected micro-world matching spec 4.1

## Spec References
- Section 3.3 (graph world)
- Section 4.1 (prototype world scope)
- Section 5.2 (directed graph topology)
- Section 9.2 (determinism)
- Section 9.10 (no teleportation)
