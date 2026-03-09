# E04ITECON-008: Conservation verification helpers

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (pure query functions on World)
**Deps**: E04ITECON-003 (ItemLot component registered)

## Problem

Spec invariant 9.5 requires that conserved quantities change only through explicit operations. A global conservation check is needed to verify that the total quantity of any commodity across all lots matches an expected value. Without this, silent creation or deletion of goods goes undetected.

## Assumption Reassessment (2026-03-09)

1. `ItemLot` component with `commodity` and `quantity: Quantity` fields exists after E04ITECON-003 — dependency
2. `World::query_item_lot()` iterator exists after E04ITECON-003 — dependency
3. `WorldError::InvariantViolation` variant exists in `error.rs` — confirmed
4. No existing conservation functions — confirmed

## Architecture Check

1. Pure query functions that scan all live `ItemLot` entities — no mutation
2. Uses `u64` accumulator to avoid overflow when summing many `Quantity` values
3. Placed in a new `conservation.rs` module to keep concerns separate
4. `Quantity` remains the lot-local semantic type; widening to `u64` happens only at the aggregate boundary

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

## Acceptance Criteria

### Tests That Must Pass

1. `total_commodity_quantity` returns 0 for a commodity with no lots
2. `total_commodity_quantity` sums across multiple lots of the same commodity
3. `total_commodity_quantity` ignores lots of different commodities
4. `total_commodity_quantity` ignores archived lots (only live lots counted)
5. `verify_conservation` returns `Ok(())` when total matches expected
6. `verify_conservation` returns `Err(InvariantViolation)` when total does not match
7. `Waste` is included in conservation checks (spec 7.1: waste is conserved)
8. After `split_lot`, `verify_conservation` still passes with original total
9. After `merge_lots`, `verify_conservation` still passes with original total
10. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Conservation helper is intentionally global — does not care about location (spec says so)
2. Uses `u64` accumulator to prevent overflow
3. Only counts live lots, not archived ones
4. All existing tests continue to pass unchanged
5. Global totals widen from `Quantity` instead of storing lot counts as raw integers

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/conservation.rs` (inline `#[cfg(test)]` module) — empty world, multiple lots, filtering, archived exclusion, Waste inclusion
2. Integration tests combining split/merge with conservation verification

### Commands

1. `cargo test -p worldwake-core conservation`
2. `cargo clippy --workspace && cargo test --workspace`
