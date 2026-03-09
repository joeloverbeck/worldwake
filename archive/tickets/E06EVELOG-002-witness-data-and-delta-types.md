# E06EVELOG-002: WitnessData and Component/Relation Delta Types

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new event-log foundation types in `worldwake-sim`
**Deps**: E06EVELOG-001 (CauseRef, EventTag, VisibilitySpec exist)

## Problem

Event records need typed delta payloads that can describe actual authoritative world mutations without leaking `worldwake-core`'s internal storage layout. The spec explicitly prohibits anonymous `Any` snapshots — delta payloads must be serializable, comparable, and typed. Additionally, events need `WitnessData` to track who perceived them.

## Assumption Reassessment (2026-03-09)

1. `EntityId` exists in `worldwake-core::ids` — confirmed
2. `CommodityKind` exists in `worldwake-core::items` — confirmed
3. `Quantity` exists in `worldwake-core::numerics` — confirmed
4. `ReservationId` exists in `worldwake-core::ids` — confirmed
5. `FactId`, `Permille`, `TickRange`, and `ReservationRecord` are public in `worldwake-core` and available for typed delta payloads — confirmed
6. Authoritative component types are exactly: `Name`, `AgentData`, `ItemLot`, `UniqueItem`, `Container` — confirmed via `component_schema.rs`
7. Authoritative semantic relation families are: `LocatedIn`, `InTransit`, `ContainedBy`, `PossessedBy`, `OwnedBy`, `MemberOf`, `LoyalTo`, `OfficeHolder`, `HostileTo`, `KnowsFact`, `BelievesFact` — confirmed via `RelationTables` and `World` APIs
8. `EntityKind` exists in `worldwake-core::entity` — confirmed
9. `BTreeSet` is used for deterministic ordered collections — confirmed project convention

## Architecture Check

1. Delta types must be serializable enums, not trait objects or `Any` — this enables deterministic hashing, replay, and equality checks.
2. Event-log deltas should model canonical world semantics, not `RelationTables`' reverse indices or helper caches. Reverse maps such as `entities_at`, `contents_of`, `property_of`, `members_of`, `loyalty_from`, `offices_held`, `hostility_from`, and `reservations_by_entity` are derived storage details and must not appear in event payloads.
3. `WitnessData` stores `BTreeSet<EntityId>` for deterministic ordering (project convention: no HashSet in authoritative state).
4. Component deltas should capture typed before/after component snapshots now. Deferring snapshots to `WorldTxn` would force a second delta redesign in E06EVELOG-006 and make E06EVELOG-003/004 depend on knowingly incomplete event payloads.
5. Relation deltas cannot be a single `{ kind, source, target }` shape because the current model includes:
   - unary relation state (`InTransit`)
   - fact relations keyed by `FactId`
   - weighted loyalty (`Permille`)
6. Reservation deltas should carry full `ReservationRecord` values, not just `ReservationId`, so released reservations remain auditable after deletion from authoritative state.

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

`ComponentValue` — enum wrapping concrete component values:
- `Name(Name)`
- `AgentData(AgentData)`
- `ItemLot(ItemLot)`
- `UniqueItem(UniqueItem)`
- `Container(Container)`

`RelationKind` — enum listing canonical semantic relations:
- `LocatedIn`
- `InTransit`
- `ContainedBy`
- `PossessedBy`
- `OwnedBy`
- `MemberOf`
- `LoyalTo`
- `OfficeHolder`
- `HostileTo`
- `KnowsFact`
- `BelievesFact`

`RelationValue` — enum carrying typed semantic relation payloads:
- `LocatedIn { entity: EntityId, place: EntityId }`
- `InTransit { entity: EntityId }`
- `ContainedBy { entity: EntityId, container: EntityId }`
- `PossessedBy { entity: EntityId, holder: EntityId }`
- `OwnedBy { entity: EntityId, owner: EntityId }`
- `MemberOf { member: EntityId, faction: EntityId }`
- `LoyalTo { subject: EntityId, target: EntityId, strength: Permille }`
- `OfficeHolder { office: EntityId, holder: EntityId }`
- `HostileTo { subject: EntityId, target: EntityId }`
- `KnowsFact { agent: EntityId, fact: FactId }`
- `BelievesFact { agent: EntityId, fact: FactId }`

`EntityDelta`:
- `Created { entity: EntityId, kind: EntityKind }`
- `Archived { entity: EntityId, kind: EntityKind }`

`ComponentDelta`:
- `Set { entity: EntityId, component_kind: ComponentKind, before: Option<ComponentValue>, after: ComponentValue }`
- `Removed { entity: EntityId, component_kind: ComponentKind, before: ComponentValue }`

`RelationDelta`:
- `Added { relation_kind: RelationKind, relation: RelationValue }`
- `Removed { relation_kind: RelationKind, relation: RelationValue }`

`QuantityDelta`:
- `Changed { entity: EntityId, commodity: CommodityKind, before: Quantity, after: Quantity }`

`ReservationDelta`:
- `Created { reservation: ReservationRecord }`
- `Released { reservation: ReservationRecord }`

### 3. Register modules in `crates/worldwake-sim/src/lib.rs`

Add `mod witness;` and `mod delta;`, re-export types.

## Files to Touch

- `crates/worldwake-sim/src/witness.rs` (new)
- `crates/worldwake-sim/src/delta.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — register modules, re-export types)

## Out of Scope

- `StateDelta` wrapper enum (E06EVELOG-003)
- `EventRecord` struct (E06EVELOG-003)
- Witness resolution logic (determining who sees what — future perception epic E14)
- EventLog storage (E06EVELOG-004)
- Journal capture from `WorldTxn` (E06EVELOG-006) — this ticket defines the data model only

## Acceptance Criteria

### Tests That Must Pass

1. `WitnessData` stores witnesses in deterministic `BTreeSet` order
2. `WitnessData` with empty sets is valid (Hidden events have no witnesses)
3. `ComponentKind` has one variant per authoritative component type in `ComponentTables`
4. `ComponentValue` has one variant per authoritative component type and can report its matching `ComponentKind`
5. `RelationKind` has one variant per canonical semantic relation family, including `InTransit`
6. `RelationValue` covers the actual relation payload shapes used by the current `World` API, including `FactId` and `Permille`
7. `EntityDelta::Created` and `EntityDelta::Archived` store both `EntityId` and `EntityKind`
8. `ComponentDelta` stores typed before/after snapshots without `Any` or stringly typed payloads
9. `QuantityDelta::Changed` stores before/after `Quantity` values
10. `ReservationDelta` variants store full `ReservationRecord` values
11. All types satisfy `Clone + Eq + Debug + Serialize + Deserialize`
12. All types survive bincode round-trip for every variant
13. Existing suite: `cargo test --workspace`

### Invariants

1. Delta kinds are typed enums, not strings (spec requirement)
2. Witness sets use `BTreeSet` for deterministic ordering (determinism invariant)
3. No `Any`, `Box<dyn>`, or type-erased values in delta payloads
4. Event deltas record semantic state only; reverse indices and other derived relation caches remain out of the event schema

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/witness.rs` — construction, deterministic ordering, empty validity, bincode round-trip
2. `crates/worldwake-sim/src/delta.rs` — kind/value completeness, component snapshot coverage, semantic relation payload coverage, bincode round-trip per variant

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `WitnessData` in `worldwake-sim` with deterministic `BTreeSet` witness storage.
  - Added typed event delta foundations in `worldwake-sim`: `ComponentKind`, `ComponentValue`, `RelationKind`, `RelationValue`, `EntityDelta`, `ComponentDelta`, `RelationDelta`, `QuantityDelta`, and `ReservationDelta`.
  - Re-exported the new types from `crates/worldwake-sim/src/lib.rs`.
  - Added focused unit coverage for witness ordering, kind/value completeness, typed snapshot payloads, semantic relation payloads, and bincode round-trips.
- Deviations from original plan:
  - Corrected the ticket before implementation because the original assumptions were lossy in three places: entity deltas lacked `EntityId`, reservation deltas only stored `ReservationId`, and relation deltas assumed every relation was a simple `EntityId -> EntityId` edge.
  - Implemented typed component before/after snapshots in this ticket instead of deferring them to `WorldTxn`, because the deferred design would have forced a later schema break across E06EVELOG-003/004/006.
- Verification results:
  - `cargo test -p worldwake-sim` passed
  - `cargo fmt --check` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
