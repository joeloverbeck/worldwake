# E04ITECON-008: Conservation verification helpers

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Additive public API only (new pure helper module; no ECS schema or lifecycle changes)
**Deps**: E03ENTSTO-005 (live query API), E04ITECON-003 (ItemLot component), E04ITECON-006 (split/merge lot algebra)

## Problem

Spec invariant 9.5 requires that conserved quantities change only through explicit operations. A global conservation check is needed to verify that the total quantity of any commodity across all lots matches an expected value. Without this, silent creation or deletion of goods goes undetected.

## Assumption Reassessment (2026-03-09)

1. `ItemLot` already exists in `crates/worldwake-core/src/items.rs` with `commodity: CommodityKind` and `quantity: Quantity` — confirmed
2. `World::query_item_lot()` already exists and returns live lots only; archived lots remain stored internally but are intentionally excluded from the public query surface — confirmed
3. `WorldError::InvariantViolation` already exists in `crates/worldwake-core/src/error.rs` — confirmed
4. `World::split_lot` and `World::merge_lots` already exist in `crates/worldwake-core/src/world.rs`, with tests covering quantity preservation and archived-source semantics — confirmed
5. `crates/worldwake-core/src/lib.rs` already follows the pattern of exposing focused helper modules such as `load.rs`; this ticket should mirror that shape instead of adding ad hoc world methods — confirmed
6. No existing conservation helper module or public totaling function exists yet — confirmed

## Architecture Check

1. Conservation belongs in a dedicated pure helper module, not on `World`. This matches the current architecture used by `load.rs`: `World` owns lifecycle and storage, helper modules own deterministic derived calculations.
2. The helper should consume `World::query_item_lot()` so it automatically inherits the live-only contract instead of duplicating archived-entity filtering logic.
3. Aggregate totals should widen from `Quantity` to `u64` only at the summation boundary. Lot-local arithmetic and public lot APIs should remain typed as `Quantity`.
4. The ticket should not introduce aliasing or compatibility shims. If later systems need richer conservation reporting, that should build on top of this helper rather than contorting this API now.
5. A minimal pair of functions is justified today. A broader aggregate report such as `totals_by_commodity()` could become valuable later, but only once there is a concrete consumer; adding it preemptively would be speculative surface area.

## What to Change

### 1. Create `crates/worldwake-core/src/conservation.rs`

```rust
pub fn total_commodity_quantity(world: &World, commodity: CommodityKind) -> u64 {
    world
        .query_item_lot()
        .filter(|(_, lot)| lot.commodity == commodity)
        .map(|(_, lot)| u64::from(lot.quantity.0))
        .sum()
}

pub fn verify_conservation(
    world: &World,
    commodity: CommodityKind,
    expected_total: u64,
) -> Result<(), WorldError> {
    let actual = total_commodity_quantity(world, commodity);
    if actual != expected_total {
        return Err(WorldError::InvariantViolation(format!(
            "conservation violation for {:?}: expected {}, found {}",
            commodity, expected_total, actual
        )));
    }
    Ok(())
}
```

Rules:
- implemented as free functions in `conservation.rs`, not as inherent `World` methods
- driven entirely by `world.query_item_lot()` so only live lots count
- no mutation, archival, provenance, or event-log changes

### 2. Register module in `lib.rs`

Add `pub mod conservation;` and re-export both functions.

## Files to Touch

- `crates/worldwake-core/src/conservation.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Per-container conservation (only global check)
- Location-aware conservation (doesn't care where lots are)
- Unique item conservation (unique items are indivisible, not quantity-tracked)
- Automatic conservation enforcement (this is a verification helper, not a constraint system)
- Production/consumption/spoilage operations (future systems epics)
- Re-implementing split/merge behavior already delivered by E04ITECON-006
- Aggregated reporting for all commodities at once

## Acceptance Criteria

### Tests That Must Pass

1. `total_commodity_quantity` returns 0 for a commodity with no lots
2. `total_commodity_quantity` sums across multiple lots of the same commodity
3. `total_commodity_quantity` ignores lots of different commodities
4. `total_commodity_quantity` ignores archived lots (only live lots counted)
5. `verify_conservation` returns `Ok(())` when total matches expected
6. `verify_conservation` returns `Err(InvariantViolation)` when total does not match
7. `Waste` is included in conservation checks (spec 7.1: waste is conserved)
8. After `split_lot`, `verify_conservation` still passes with the original total because both resulting live lots are counted
9. After `merge_lots`, `verify_conservation` still passes with the original total and the archived source lot is not double-counted
10. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Conservation helper is intentionally global — does not care about location (spec says so)
2. Uses `u64` accumulator to prevent overflow
3. Only counts live lots, not archived ones
4. All existing split/merge semantics remain unchanged; the helper observes them rather than re-encoding them
5. Global totals widen from `Quantity` instead of storing lot counts as raw integers
6. The API shape stays additive and composable with later E05/E06 work instead of embedding conservation checks into `World` mutation paths prematurely

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/conservation.rs` (inline `#[cfg(test)]` module) — empty world, multiple lots, commodity filtering, archived exclusion, mismatch errors, Waste inclusion, and conservation across split/merge

### Commands

1. `cargo test -p worldwake-core conservation`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `crates/worldwake-core/src/conservation.rs` with `total_commodity_quantity` and `verify_conservation`
  - Re-exported the conservation helpers from `crates/worldwake-core/src/lib.rs`
  - Added focused conservation tests for empty totals, live-only aggregation, mismatch reporting, and split/merge preservation with archived-source exclusion
- Deviations from original plan:
  - Corrected the ticket before implementation to reflect that `query_item_lot`, `split_lot`, `merge_lots`, and live-only lot-query behavior already existed
  - Kept the design as a dedicated helper module rather than adding methods on `World`, matching the existing `load.rs` architecture
  - Consolidated the split/merge verification into the conservation module’s inline tests instead of introducing a separate integration harness
- Verification results:
  - `cargo test -p worldwake-core conservation` passed
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
