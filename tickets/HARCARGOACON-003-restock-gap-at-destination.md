# HARCARGOACON-003: Add restock_gap_at_destination helper in enterprise.rs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-ai (enterprise module)
**Deps**: HARCARGOACON-002 (destination-aware read helpers must exist on BeliefView)

## Problem

The existing `restock_gap_for_market` in `enterprise.rs` computes restock gap using `commodity_quantity(agent, commodity)` — total agent stock everywhere. Cargo candidate generation and goal satisfaction need a destination-local variant that computes gap using `controlled_commodity_quantity_at_place(agent, destination, commodity)` to know how much stock is missing specifically at the destination market.

## Assumption Reassessment (2026-03-12)

1. `restock_gap_for_market` exists at `enterprise.rs:86-99` — confirmed
2. It uses `commodity_quantity(agent, commodity)` for current_stock — confirmed at line 97
3. `relevant_demand_quantity` helper at `enterprise.rs:73-84` computes observed demand — confirmed; this is reusable
4. The spec explicitly says: do NOT modify `restock_gap_for_market` — confirmed per spec Section B

## Architecture Check

1. New function alongside existing one is cleaner than adding a parameter to the existing function — the two have different semantics (global stock vs. destination-local stock)
2. Reuses existing `relevant_demand_quantity` helper unchanged

## What to Change

### 1. Add `restock_gap_at_destination` function

Add a new public function in `enterprise.rs`:

```rust
pub fn restock_gap_at_destination(
    view: &dyn BeliefView,
    agent: EntityId,
    destination: EntityId,
    commodity: CommodityKind,
) -> Option<Quantity> {
    let observed_quantity = relevant_demand_quantity(view, agent, destination, commodity);
    if observed_quantity == 0 {
        return None;
    }
    let current_stock_at_dest = view
        .controlled_commodity_quantity_at_place(agent, destination, commodity)
        .0;
    (current_stock_at_dest < observed_quantity)
        .then_some(Quantity(observed_quantity - current_stock_at_dest))
}
```

Key difference from `restock_gap_for_market`: uses `controlled_commodity_quantity_at_place` instead of `commodity_quantity`.

### 2. Export the function

Ensure `restock_gap_at_destination` is `pub` and accessible from `candidate_generation.rs` and `goal_model.rs`.

## Files to Touch

- `crates/worldwake-ai/src/enterprise.rs` (modify — add function)

## Out of Scope

- Modifying existing `restock_gap_for_market` (must remain unchanged)
- Modifying existing `relevant_demand_quantity` (reused as-is)
- Using the new helper in candidate generation (HARCARGOACON-004)
- Using the new helper in goal satisfaction (HARCARGOACON-005)
- Any changes to `BeliefView` trait (done in HARCARGOACON-002)

## Acceptance Criteria

### Tests That Must Pass

1. New test: `restock_gap_at_destination` returns `Some(gap)` when destination stock is below observed demand
2. New test: `restock_gap_at_destination` returns `None` when no demand observations exist for the destination
3. New test: `restock_gap_at_destination` returns `None` when destination stock meets or exceeds observed demand
4. New test: `restock_gap_at_destination` uses destination-local stock, not global agent stock (agent has commodity elsewhere but not at destination → gap exists)
5. Existing `restock_gap_for_market` tests remain unchanged and passing
6. `cargo test --workspace` and `cargo clippy --workspace` pass

### Invariants

1. `restock_gap_for_market` is unchanged — its behavior and callers are not modified
2. `restock_gap_at_destination` only considers stock at the specified destination place
3. Both functions use the same `relevant_demand_quantity` helper for demand observation

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/enterprise.rs` — `restock_gap_at_destination_returns_gap_when_understocked_at_destination`, `restock_gap_at_destination_returns_none_when_no_demand`, `restock_gap_at_destination_returns_none_when_fully_stocked`, `restock_gap_at_destination_ignores_stock_at_other_places`

### Commands

1. `cargo test -p worldwake-ai enterprise`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
