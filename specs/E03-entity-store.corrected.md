# E03: Entity Store, Typed Component Tables & Mutation Surface

## Epic Summary
Implement the authoritative `World` model with deterministic entity allocation, explicit typed component tables, entity lifecycle, and read/query APIs.

This epic must **not** build a faux-generic ECS on top of `TypeId` and `Box<dyn Any>`. That approach is attractive at first and then becomes poison for save/load, stable hashing, event diffing, and replay.

## Phase
Phase 1: World Legality

## Crate
`worldwake-core`

## Dependencies
- E01 (deterministic core types)
- E02 (topology types composed into the world model)

## Why this revision exists
The original version proposed `HashMap<TypeId, HashMap<EntityId, Box<dyn Any>>>` as an option. That is the wrong foundation for this project.

For Phase 1 we need:
- deterministic iteration
- explicit serialization
- typed before/after deltas for events
- clean save/load
- no hidden runtime type registry

That pushes the design toward explicit typed tables.

## Deliverables

### EntityKind
Introduce an explicit kind for every entity so invariants can reason about physicality and later system rules.

Minimum cases:
- `Agent`
- `ItemLot`
- `UniqueItem`
- `Container`
- `Facility`
- `Place`
- `Faction`
- `Office`
- `Contract`
- `Rumor`
- `EventRecordProxy` (optional helper only if event entities are actually materialized)

`EntityMeta`:
- `kind: EntityKind`
- `created_at: Tick`
- `archived_at: Option<Tick>`

### World Struct
`World` owns the authoritative entity metadata and component tables.

Required fields:
- deterministic entity allocator
- `entities: BTreeMap<EntityId, EntityMeta>`
- `topology: Topology`
- explicit typed component tables grouped in a `ComponentTables` struct

### ComponentTables
Use one ordered table per component type:
- `BTreeMap<EntityId, T>` for single-instance components
- `BTreeMap<EntityId, Vec<T>>` only when multiplicity is intentional and documented

Examples of Phase 1 component tables that should exist by the end of E05:
- `names`
- `agents`
- `places`
- `containers`
- `item_lots`
- `unique_items`
- `offices`
- `facilities`
- any other authoritative components introduced by E04/E05

To reduce boilerplate, internal macros are allowed, but the resulting storage is still explicit and typed.

### Entity Allocator
Provide:
- `create_entity(kind: EntityKind, created_at: Tick) -> EntityId`
- `archive_entity(id: EntityId, tick: Tick) -> Result<()>`
- `purge_entity(id: EntityId) -> Result<()>` only for fully cleaned-up entities if needed later
- `is_alive(id) -> bool`
- `is_archived(id) -> bool`

Rules:
- stale ids must be detected by generation
- archival does not silently delete authoritative history
- any slot reuse increments generation

### Component API
Provide deterministic typed APIs:
- `insert_component<T>(entity, value) -> Result<()>`
- `get_component<T>(entity) -> Option<&T>`
- `get_component_mut<T>(entity) -> Option<&mut T>` only inside restricted world-editing paths
- `remove_component<T>(entity) -> Result<Option<T>>`
- `has_component<T>(entity) -> bool`

Critical rule:
- ordinary simulation code must not receive broad mutable access to all tables
- authoritative mutation must flow through narrow APIs that E06 can journal

### Query API
Required deterministic queries:
- `entities() -> impl Iterator<Item = EntityId>` in sorted id order
- `entities_with<T>() -> impl Iterator<Item = EntityId>`
- `entities_with_component<T>() -> impl Iterator<Item = (EntityId, &T)>`
- multi-component intersection queries with deterministic ordering
- filtered queries that preserve base ordering

### Factory / Archetype Helpers
Keep convenience helpers, but do not hide authoritative state changes.
Examples:
- `create_agent(name, control_source, tick) -> EntityId`
- `create_place(name, tags, tick) -> EntityId`
- `create_office(name, tick) -> EntityId`

These helpers should just be thin wrappers over normal entity + component insertion.

### Mutation Surface for Later Event Journaling
E03 must prepare for E06 by drawing a hard line between:
- read-only world access
- controlled authoritative mutation

Required design rule:
- no public fields on `World`
- no direct external mutation of component tables
- every persistent mutation path is narrow enough to be wrapped by a journal in E06

## Invariants Enforced
- authoritative state is deterministic and serializable
- archived entities remain distinguishable from live entities
- no runtime-erased `Any` store exists in authoritative world state
- component queries return stable ordering

## Tests
- [ ] CRUD: insert, get, update, remove component round-trips correctly
- [ ] Entity creation returns unique ids
- [ ] Archival marks an entity non-live without aliasing stale ids
- [ ] Slot reuse increments generation
- [ ] Removing a component makes subsequent typed queries return `None`
- [ ] Query APIs return only entities with matching components
- [ ] Multi-component intersection works correctly
- [ ] Query iteration order is deterministic
- [ ] World serializes and deserializes correctly
- [ ] No `TypeId`, `Any`, or trait-object component storage appears in authoritative world code

## Acceptance Criteria
- clean typed API without `unsafe`
- no external ECS crate dependency
- explicit component tables instead of runtime-erased storage
- authoritative world state is ready for event journaling, save/load, and stable hashing

## Spec References
- Section 5.3 (entity classes)
- Section 9.1 (simulation authority)
- Section 9.2 (determinism)
- Section 9.19 (save/load integrity)
