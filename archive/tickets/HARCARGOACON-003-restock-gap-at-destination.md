# HARCARGOACON-003: Add destination-local restock gap helper in enterprise.rs

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-ai (enterprise module)
**Deps**: None

## Problem

The existing `restock_gap_for_market` in `enterprise.rs` computes restock gap using `commodity_quantity(agent, commodity)`, which counts total agent stock everywhere. That remains valid for current enterprise pressure/scoring, but it is the wrong primitive for destination-sensitive logistics logic. A second helper is needed that computes the gap using `controlled_commodity_quantity_at_place(agent, destination, commodity)` so downstream cargo logic can reason about stock that is actually present at the destination.

## Assumption Reassessment (2026-03-12)

1. `restock_gap_for_market` exists in `crates/worldwake-ai/src/enterprise.rs` and still uses global `commodity_quantity(agent, commodity)` for current stock.
2. `relevant_demand_quantity` already exists and is the right shared demand-input helper for both gap calculations.
3. `controlled_commodity_quantity_at_place(agent, destination, commodity)` already exists on `BeliefView` and `PlanningState`; this ticket is not blocked on HARCARGOACON-002 anymore.
4. `GoalKind::MoveCargo` has already been migrated to `{ commodity, destination }`, and related ranking/goal-key changes already exist in the current codebase.
5. `enterprise.rs` is in a private module (`mod enterprise;` in `lib.rs`), so this helper only needs crate visibility for current architecture. It does not need public re-exporting from the crate root.
6. The hardening spec still explicitly says not to change `restock_gap_for_market`; this ticket should add a second helper, not broaden or repurpose the existing one.

## Architecture Check

1. A separate helper is cleaner than parameterizing `restock_gap_for_market`. The two functions encode different semantics: global stock pressure versus destination-local stock sufficiency.
2. Reusing `relevant_demand_quantity` preserves one demand interpretation and avoids duplicating observation aggregation.
3. This ticket should remain helper-focused. Candidate generation and goal satisfaction changes belong to later tickets/spec work that already assumes the broader cargo-goal redesign.

## What to Change

### 1. Add `restock_gap_at_destination` function

Add a new `pub(crate)` function in `enterprise.rs`:

```rust
pub(crate) fn restock_gap_at_destination(
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

Key difference from `restock_gap_for_market`: it uses `controlled_commodity_quantity_at_place` instead of `commodity_quantity`.

### 2. Add focused unit coverage

Add unit tests in `enterprise.rs` that lock down the new destination-local semantics and prove it does not collapse back to global stock counting.

## Files to Touch

- `crates/worldwake-ai/src/enterprise.rs` (modify — add helper and tests)

## Out of Scope

- Modifying existing `restock_gap_for_market` (must remain unchanged)
- Modifying existing `relevant_demand_quantity` (reused as-is)
- Re-exporting enterprise internals from `worldwake-ai/src/lib.rs`
- Using the new helper in candidate generation (HARCARGOACON-004)
- Using the new helper in goal satisfaction (HARCARGOACON-005)
- Any changes to `BeliefView` or `PlanningState` destination-aware quantity helpers (already present)
- Reopening the broader `MoveCargo` identity/ranking/search architecture already covered by the hardening spec

## Acceptance Criteria

### Tests That Must Pass

1. New test: `restock_gap_at_destination` returns `Some(gap)` when destination stock is below observed demand
2. New test: `restock_gap_at_destination` returns `None` when no demand observations exist for the destination
3. New test: `restock_gap_at_destination` returns `None` when destination stock meets or exceeds observed demand
4. New test: `restock_gap_at_destination` uses destination-local stock, not global agent stock (agent has commodity elsewhere but not at destination → gap exists)
5. Existing enterprise-related tests remain passing
6. `cargo test --workspace` and `cargo clippy --workspace` pass

### Invariants

1. `restock_gap_for_market` is unchanged — its behavior and callers are not modified
2. `restock_gap_at_destination` only considers stock at the specified destination place
3. Both functions use the same `relevant_demand_quantity` helper for demand observation
4. No aliasing, compatibility wrapper, or dual-semantics helper is introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/enterprise.rs` — `restock_gap_at_destination_returns_gap_when_understocked_at_destination`
2. `crates/worldwake-ai/src/enterprise.rs` — `restock_gap_at_destination_returns_none_when_no_demand`
3. `crates/worldwake-ai/src/enterprise.rs` — `restock_gap_at_destination_returns_none_when_fully_stocked`
4. `crates/worldwake-ai/src/enterprise.rs` — `restock_gap_at_destination_ignores_stock_at_other_places`

### Commands

1. `cargo test -p worldwake-ai enterprise`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - Added `restock_gap_at_destination` to `crates/worldwake-ai/src/enterprise.rs`
  - Added focused unit tests in `enterprise.rs` covering understocked, no-demand, fully-stocked, and remote-stock edge cases
  - Reassessed and corrected the ticket scope to reflect that destination-aware belief helpers and commodity-based `MoveCargo` identity already existed
- Deviations from original plan:
  - The helper was added with `pub(crate)` visibility rather than crate-public export because `enterprise.rs` remains an internal module and no current production call site in this ticket requires broader exposure
  - No candidate-generation or goal-model integration was added here; that broader cargo usage remains follow-on work
  - A local `#[allow(dead_code)]` was added to avoid introducing an unused-helper warning before those follow-on integrations land
- Verification results:
  - `cargo test -p worldwake-ai enterprise` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
