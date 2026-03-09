# E04ITECON-002: LotOperation enum and ProvenanceEntry struct

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E04ITECON-001 (CommodityKind and TradeCategory — completed)

## Problem

Lot lineage tracking requires a `ProvenanceEntry` that records when, why, and from where a lot was created, split, merged, or otherwise transformed. Without provenance, conservation violations become invisible and lot history is lost.

## Assumption Reassessment (2026-03-09)

1. `Tick`, `EventId`, and `EntityId` already exist in `ids.rs` and satisfy the serialization/ordering bounds this ticket needs — confirmed
2. `Quantity` already exists in `numerics.rs` and should be reused for conserved lot amounts instead of raw `u32` — confirmed
3. `crates/worldwake-core/src/items.rs` already exists from archived E04ITECON-001 and is the canonical home for item-domain enums — confirmed
4. `crates/worldwake-core/src/lib.rs` already re-exports item-domain types from `items.rs` — confirmed
5. No existing provenance types are defined yet in `worldwake-core` — confirmed
6. `items.rs` already uses inline unit tests plus canonical `ALL` variant arrays for item enums, so provenance enum coverage should follow that pattern instead of duplicating variant inventories in tests — confirmed

## Architecture Check

1. Pure data types belong in `items.rs` alongside `CommodityKind` and `TradeCategory`; they should not be split into `components.rs` or world-facing APIs yet
2. Append-only provenance remains a call-site invariant; these types model lineage facts but do not enforce mutation policy themselves
3. `LotOperation` should expose a canonical `ALL` array, matching the established enum pattern in `items.rs`, so tests and later lot logic share one source of truth
4. Provenance quantities should use the semantic `Quantity` wrapper so split/merge/production APIs stay type-safe at the conserved-stock boundary

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

impl LotOperation {
    pub const ALL: [Self; 8] = [
        Self::Created,
        Self::Split,
        Self::Merge,
        Self::Produced,
        Self::Consumed,
        Self::Destroyed,
        Self::Spoiled,
        Self::Transformed,
    ];
}
```

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    pub tick: Tick,
    pub event_id: Option<EventId>,
    pub operation: LotOperation,
    pub related_lot: Option<EntityId>,
    pub amount: Quantity,
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

1. `LotOperation::ALL` contains all 8 variants in declaration order and each variant round-trips through bincode
2. `LotOperation` satisfies `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned`
3. `ProvenanceEntry` round-trips through bincode (with `Some` and `None` event_id/related_lot)
4. `ProvenanceEntry` satisfies `Clone + Debug + Eq + Serialize + DeserializeOwned`
5. `LotOperation` ordering is deterministic (`LotOperation::ALL` remains unchanged after reverse + sort)
6. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `ProvenanceEntry` uses no `HashMap` or `HashSet`
2. All `LotOperation` variants match the spec's list exactly
3. Conserved provenance amounts use `Quantity`, not raw integers

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` (extend `#[cfg(test)]` module) — canonical variant list, bincode round-trips, trait bounds, ordering, `ProvenanceEntry` optional-field coverage

### Commands

1. `cargo test -p worldwake-core items`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Outcome amended: 2026-03-09
- Completion date: 2026-03-09
- What actually changed:
  - Added `LotOperation` and `ProvenanceEntry` to `crates/worldwake-core/src/items.rs`
  - Added canonical `LotOperation::ALL` coverage to match the established item-enum pattern and keep future lot logic/tests DRY
  - Re-exported `LotOperation` and `ProvenanceEntry` from `crates/worldwake-core/src/lib.rs`
  - Added inline unit coverage for trait bounds, canonical variant inventory, deterministic ordering, and bincode round-trips including `Option` field cases
  - Refined `ProvenanceEntry` from `source_lot` to `related_lot` once E04ITECON-006 made the asymmetry concrete; this keeps provenance neutral across split and merge operations and better fits future lot transforms
- Deviations from original plan:
  - Strengthened the enum design during reassessment by adding `LotOperation::ALL`; this was not explicit in the original draft but better matches the current `items.rs` architecture and reduces future test drift
  - Tightened the ticket assumptions to reflect that `items.rs` and the E04ITECON-001 dependency were already completed rather than still pending
  - The archived original field name was superseded by a stronger relationship-oriented field name after lot algebra landed; the old wording is retained here only where needed to explain the amendment
- Verification results:
  - `cargo fmt --all` passed
  - `cargo test -p worldwake-core items` passed
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
