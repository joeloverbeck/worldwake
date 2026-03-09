# E06EVELOG-002: WitnessData and Component/Relation Delta Types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new types in `worldwake-sim`, new enums for component/relation kinds
**Deps**: E06EVELOG-001 (CauseRef, EventTag, VisibilitySpec exist)

## Problem

Event records need typed delta payloads to capture before/after snapshots of world mutations. The spec explicitly prohibits anonymous `Any` snapshots — delta kinds must be typed enums. Additionally, events need `WitnessData` to track who perceived them.

## Assumption Reassessment (2026-03-09)

1. `EntityId` exists in `worldwake-core::ids` — confirmed
2. `CommodityKind` exists in `worldwake-core::items` — confirmed
3. `Quantity` exists in `worldwake-core::numerics` — confirmed
4. `ReservationId` exists in `worldwake-core::ids` — confirmed
5. Component types are: `Name`, `AgentData`, `ItemLot`, `UniqueItem`, `Container` — confirmed via `component_schema.rs`
6. Relation kinds include: located_in, contained_by, possessed_by, owned_by, member_of, loyal_to, office_holder, hostile_to, knows_fact, believes_fact, reservations — confirmed via `RelationTables`
7. `EntityKind` exists in `worldwake-core::entity` — confirmed
8. `BTreeSet` is used for deterministic ordered collections — confirmed project convention

## Architecture Check

1. Delta types must be serializable enums, not trait objects or `Any` — this enables deterministic hashing and replay
2. `ComponentKind` and `RelationKind` enums enumerate the concrete component/relation types from worldwake-core, keeping delta payloads typed
3. `WitnessData` stores `BTreeSet<EntityId>` for deterministic ordering (project convention: no HashSet in authoritative state)
4. Before/after values in `ComponentDelta::Set` use a `ComponentValue` enum wrapping concrete component types, avoiding downcasting

## What to Change

### 1. Create `crates/worldwake-sim/src/witness.rs`

Define `WitnessData`:
```rust
pub struct WitnessData {
    pub direct_witnesses: BTreeSet<EntityId>,
    pub potential_witnesses: BTreeSet<EntityId>,
}
```

### 2. Create `crates/worldwake-sim/src/delta.rs`

Define typed delta enums:

`ComponentKind` — enum listing: `Name`, `AgentData`, `ItemLot`, `UniqueItem`, `Container`

`RelationKind` — enum listing: `LocatedIn`, `ContainedBy`, `PossessedBy`, `OwnedBy`, `MemberOf`, `LoyalTo`, `OfficeHolder`, `HostileTo`, `KnowsFact`, `BelievesFact`, `InTransit`

`EntityDelta`:
- `Created { kind: EntityKind }`
- `Archived { kind: EntityKind }`

`ComponentDelta`:
- `Set { entity: EntityId, component_kind: ComponentKind }`
- `Removed { entity: EntityId, component_kind: ComponentKind }`

Note: before/after snapshots are deferred — the initial implementation tracks which component changed on which entity. Full before/after value capture will be added in E06EVELOG-006 when `WorldTxn` records deltas during mutation.

`RelationDelta`:
- `Added { kind: RelationKind, source: EntityId, target: EntityId }`
- `Removed { kind: RelationKind, source: EntityId, target: EntityId }`

`QuantityDelta`:
- `Changed { entity: EntityId, commodity: CommodityKind, before: Quantity, after: Quantity }`

`ReservationDelta`:
- `Created { id: ReservationId }`
- `Released { id: ReservationId }`

### 3. Register modules in `crates/worldwake-sim/src/lib.rs`

Add `mod witness;` and `mod delta;`, re-export types.

## Files to Touch

- `crates/worldwake-sim/src/witness.rs` (new)
- `crates/worldwake-sim/src/delta.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — register modules, re-export types)

## Out of Scope

- `StateDelta` wrapper enum (E06EVELOG-003)
- `EventRecord` struct (E06EVELOG-003)
- Full before/after component value snapshots (deferred to E06EVELOG-006 WorldTxn)
- Witness resolution logic (determining who sees what — future perception epic E14)
- EventLog storage (E06EVELOG-004)

## Acceptance Criteria

### Tests That Must Pass

1. `WitnessData` stores witnesses in deterministic `BTreeSet` order
2. `WitnessData` with empty sets is valid (Hidden events have no witnesses)
3. `ComponentKind` has one variant per component type in `ComponentTables`
4. `RelationKind` has one variant per relation type in `RelationTables`
5. `EntityDelta::Created` and `EntityDelta::Archived` store `EntityKind`
6. `QuantityDelta::Changed` stores before/after `Quantity` values
7. `ReservationDelta` variants store `ReservationId`
8. All types satisfy `Clone + Eq + Debug + Serialize + Deserialize`
9. All types survive bincode round-trip for every variant
10. Existing suite: `cargo test --workspace`

### Invariants

1. Delta kinds are typed enums, not strings (spec requirement)
2. Witness sets use `BTreeSet` for deterministic ordering (determinism invariant)
3. No `Any`, `Box<dyn>`, or type-erased values in delta payloads

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/witness.rs` — construction, deterministic ordering, empty validity, bincode round-trip
2. `crates/worldwake-sim/src/delta.rs` — variant construction per delta type, kind enum completeness, bincode round-trip per variant

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`
