# S05MERSTOSTALL-001: Add `StockStoragePolicy` and `StockAssignment` components

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new components in worldwake-core
**Deps**: None (S04 implemented, S01 implemented)

## Problem

The merchant stock storage spec (S05) introduces two new components that must exist before any stock-handling actions or facility-based sale visibility can be built. `StockStoragePolicy` records which containers a facility uses for storage vs display. `StockAssignment` records whether a lot is stored or displayed at a facility.

## Assumption Reassessment (2026-04-01)

1. `component_schema.rs` exists at `crates/worldwake-core/src/component_schema.rs` and uses `define_component_schema!` macro for registration. Confirmed.
2. `SaleListing` is registered in `component_schema.rs` on `EntityKind::ItemLot` — provides the pattern for `StockAssignment`. Confirmed.
3. `EntityKind::Facility` and `EntityKind::Container` variants exist in `entity.rs:12-13`. Confirmed.
4. `EntityId` is the standard entity reference type. Confirmed.
5. No `StockStoragePolicy` or `StockAssignment` types exist in the codebase. Confirmed via grep.

## Architecture Check

1. Two simple components following the exact same registration pattern as `SaleListing`. No new abstractions, no new traits — just data types and schema registration.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. `StockStoragePolicy` can be set/get on facility entities → authoritative world state (focused unit test)
2. `StockAssignment` can be set/get on ItemLot entities → authoritative world state (focused unit test)
3. Components round-trip through serialization → focused unit test using existing serde pattern

## What to Change

### 1. Add `StockStoragePolicy` type

In `crates/worldwake-core/src/`, add (in a new file or in `trade.rs` alongside `SaleListing`):

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StockStoragePolicy {
    pub stock_container: EntityId,
    pub display_container: Option<EntityId>,
}

impl Component for StockStoragePolicy {}
```

### 2. Add `StockAssignment` types

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum StockAssignmentKind {
    Stored,
    Displayed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StockAssignment {
    pub facility: EntityId,
    pub kind: StockAssignmentKind,
}

impl Component for StockAssignment {}
```

### 3. Register in `component_schema.rs`

Add both components to the `define_component_schema!` macro invocation:
- `StockStoragePolicy` — on facility-like entities
- `StockAssignment` — on `EntityKind::ItemLot`

### 4. Re-export from `lib.rs`

Add `StockStoragePolicy`, `StockAssignment`, `StockAssignmentKind` to the public exports of `worldwake-core`.

## Files to Touch

- `crates/worldwake-core/src/trade.rs` (modify — add types alongside `SaleListing`)
- `crates/worldwake-core/src/component_schema.rs` (modify — register components)
- `crates/worldwake-core/src/lib.rs` (modify — re-export)

## Out of Scope

- Action handlers that use these components (ticket 003/004)
- Belief view queries for facility stock (ticket 005)
- AI planning changes (ticket 007)
- Any facility creation helpers (ticket 002)

## Acceptance Criteria

### Tests That Must Pass

1. `StockStoragePolicy` can be set on a facility entity and retrieved
2. `StockAssignment` can be set on an ItemLot entity and retrieved
3. Both components serialize/deserialize correctly
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Component getters/setters follow the same pattern as `SaleListing`
2. No new EntityKind variants introduced — uses existing `Facility` and `Container`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/trade.rs` — focused tests for StockStoragePolicy and StockAssignment construction, equality, serialization
2. `crates/worldwake-core/src/component_schema.rs` — schema registration smoke test (existing pattern)

### Commands

1. `cargo test -p worldwake-core -- stock_storage`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome (2026-04-01)

### What changed

1. Added `StockStoragePolicy`, `StockAssignmentKind`, and `StockAssignment` types to `crates/worldwake-core/src/trade.rs`
2. Registered both components in `component_schema.rs` (`StockStoragePolicy` on `EntityKind::Facility`, `StockAssignment` on `EntityKind::ItemLot`)
3. Re-exported all three types from `lib.rs`
4. Added imports in `delta.rs`, `world.rs`, `component_tables.rs` (required by the macro expansion)
5. Added `ComponentValue` samples in `delta.rs::component_samples()` and fixed a test that depended on `.pop()` ordering
6. Added `sample_stock_storage_policy` and `sample_stock_assignment` helpers to `test_utils.rs`
7. Added 7 focused tests: component bounds, bincode roundtrip, Copy/Hash bounds, Optional display_container variant

### Deviations

- Added `Hash` derive to `StockAssignmentKind` (needed for `assert_copy_value_bounds` test pattern, consistent with `DemandObservationReason`)
- Fixed pre-existing fragile test in `delta.rs` that used `component_samples().pop()` — replaced with explicit `SaleListing` construction

### Verification

- `cargo test -p worldwake-core`: 886 passed, 0 failed
- `cargo test --workspace`: all pass, 0 failures
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
