# E04ITECON-002: LotOperation enum and ProvenanceEntry struct

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E04ITECON-001 (CommodityKind)

## Problem

Lot lineage tracking requires a `ProvenanceEntry` that records when, why, and from where a lot was created, split, merged, or otherwise transformed. Without provenance, conservation violations become invisible and lot history is lost.

## Assumption Reassessment (2026-03-09)

1. `Tick` and `EventId` and `EntityId` already exist in `ids.rs` — confirmed
2. No existing provenance types — confirmed
3. `items.rs` will exist after E04ITECON-001 — dependency

## Architecture Check

1. Pure data types with no logic beyond construction; append-only provenance is enforced at the call site, not in these types
2. Placed in `items.rs` alongside `CommodityKind` since they are part of the item identity model

## What to Change

### 1. Add to `crates/worldwake-core/src/items.rs`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum LotOperation {
    Created,
    Split,
    Merge,
    Produced,
    Consumed,
    Destroyed,
    Spoiled,
    Transformed,
}
```

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    pub tick: Tick,
    pub event_id: Option<EventId>,
    pub operation: LotOperation,
    pub source_lot: Option<EntityId>,
    pub amount: u32,
}
```

### 2. Re-export from `lib.rs`

Add `LotOperation` and `ProvenanceEntry` to re-exports.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify — add re-exports)

## Out of Scope

- `ItemLot` component (E04ITECON-003)
- Provenance enforcement logic (E04ITECON-006 — lot algebra)
- Event log integration (E06)
- Any mutation of provenance entries after creation (append-only is a call-site invariant)

## Acceptance Criteria

### Tests That Must Pass

1. All 8 `LotOperation` variants round-trip through bincode
2. `LotOperation` satisfies `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned`
3. `ProvenanceEntry` round-trips through bincode (with `Some` and `None` event_id/source_lot)
4. `ProvenanceEntry` satisfies `Clone + Debug + Eq + Serialize + DeserializeOwned`
5. `LotOperation` ordering is deterministic
6. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `ProvenanceEntry` uses no `HashMap` or `HashSet`
2. All `LotOperation` variants match the spec's list exactly

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` (extend `#[cfg(test)]` module) — bincode round-trips, trait bounds, ordering

### Commands

1. `cargo test -p worldwake-core items`
2. `cargo clippy --workspace && cargo test --workspace`
