# S05MERSTOSTALL-004: Add stage_stock_for_sale and unstage_stock actions

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action definitions and handlers in worldwake-systems
**Deps**: S05MERSTOSTALL-003

## Problem

Merchants need actions to move goods between storage and display, with `SaleListing` lifecycle management. Without staging/unstaging, goods in stock containers cannot become visible for sale.

## Assumption Reassessment (2026-04-01)

1. `store_stock` and `collect_display_stock` actions exist — assumed via S05MERSTOSTALL-003 dependency.
2. `StockStoragePolicy` has `display_container: Option<EntityId>` — confirmed via S05MERSTOSTALL-001 outcome.
3. `SaleListing` component exists and is registered on `EntityKind::ItemLot` — confirmed in `component_schema.rs`.
4. `StockAssignmentKind::Displayed` variant exists — confirmed via S05MERSTOSTALL-001.
5. Display container may be `None` on some facilities — staging must fail gracefully if no display container.

## Architecture Check

1. Stage/unstage follow the same action handler pattern as store/collect from ticket 003. They extend `stock_actions.rs` rather than creating a new module — keeps related actions co-located.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. `stage_stock_for_sale` moves lot from stock→display container + adds `SaleListing` + sets `Displayed` → event-log delta + authoritative world state (focused test)
2. `unstage_stock` reverses: display→stock container + clears `SaleListing` + sets `Stored` → event-log delta + authoritative world state (focused test)
3. No display container → staging fails gracefully → action trace (focused test)
4. Round-trip store→stage→unstage→collect preserves item → conservation test

## What to Change

### 1. Add staging actions to stock_actions.rs

In `crates/worldwake-systems/src/stock_actions.rs`, add:
- `stage_stock_for_sale`: validates lot is in stock container (StockAssignment = Stored), facility has display_container. Moves lot to display container, adds `SaleListing`, sets `StockAssignment { facility, kind: Displayed }`.
- `unstage_stock`: validates lot is in display container (StockAssignment = Displayed). Moves lot to stock container, removes `SaleListing`, sets `StockAssignment { facility, kind: Stored }`.

### 2. Register in action_registry.rs

Add both actions to the action registry.

## Files to Touch

- `crates/worldwake-systems/src/stock_actions.rs` (modify)
- `crates/worldwake-systems/src/action_registry.rs` (modify)

## Out of Scope

- Sale visibility evolution in belief views (005)
- AI planning for staging (007)
- MoveCargo evolution (006)
- Theft distinction (008)

## Acceptance Criteria

### Tests That Must Pass

1. `stage_stock_for_sale` moves lot to display container, adds `SaleListing`, sets `Displayed`
2. `unstage_stock` moves lot to stock container, clears `SaleListing`, sets `Stored`
3. Staging fails when facility has no display container
4. Round-trip store→stage→unstage→collect preserves item and conservation
5. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Items cannot be created/destroyed — conservation enforced
2. `SaleListing` only exists on lots with `StockAssignment::Displayed`
3. `StockAssignment` always matches the lot's actual container location

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/stock_actions.rs` — stage adds SaleListing, moves to display, sets Displayed
2. `crates/worldwake-systems/src/stock_actions.rs` — unstage reverses staging completely
3. `crates/worldwake-systems/src/stock_actions.rs` — no display container fails gracefully
4. `crates/worldwake-systems/src/stock_actions.rs` — full round-trip conservation test

### Commands

1. `cargo test -p worldwake-systems -- stock`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome (2026-04-01)

### What changed

1. Added `stage_stock_for_sale` handlers to `stock_actions.rs` — validates lot is Stored and facility has display_container, moves lot from stock→display container, sets `StockAssignment { kind: Displayed }`, adds `SaleListing { listed_at: txn.tick() }`
2. Added `unstage_stock` handlers — validates lot is Displayed, moves from display→stock container, sets `StockAssignment { kind: Stored }`, clears `SaleListing`
3. Registered both in `register_stock_actions` (now 4 actions total: store, collect, stage, unstage)
4. Used `ActionDomain::Trade` for stage/unstage (vs `Transport` for store/collect) since staging is trade-domain activity
5. Added `DisplayTestHarness` with facility that has both stock and display containers
6. 4 new tests: stage moves + adds listing, unstage reverses, no-display fails, full round-trip conservation

### Deviations

None — implementation matches ticket exactly.

### Verification

- `cargo test -p worldwake-systems`: 445 passed, 0 failed (436 baseline + 9 stock action tests)
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
