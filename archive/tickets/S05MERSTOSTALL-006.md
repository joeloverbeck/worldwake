# S05MERSTOSTALL-006: Evolve MoveCargo for merchant restock to target facility storage

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — MoveCargo facility-custody terminal, planner transitions, and store_stock affordance alignment
**Deps**: S05MERSTOSTALL-005

## Problem

Merchant restock via `MoveCargo` should target the facility's `stock_container`, not mere arrival-with-possession. When a destination has a `StockStoragePolicy`, the terminal condition must be "stock is in the facility's containers," not "carrier arrived at the place."

## Assumption Reassessment (2026-04-01)

1. `MoveCargo` goal/action exists in the planner — confirmed. Live AI still models `MoveCargo` around destination-local controlled stock, not explicit facility stock custody.
2. `StockStoragePolicy` is queryable on facility entities — confirmed via S05MERSTOSTALL-001 outcome.
3. Sale visibility evolution (005) is complete — facility-based model is the active paradigm.
4. `store_stock` exists as authoritative stock-management action, but current planning does not route `MoveCargo` through it for facility destinations.
5. Non-merchant `MoveCargo` (destinations without `StockStoragePolicy`) must remain unchanged — behavioral branch, not replacement.
6. The shared abstraction boundary under audit is merchant destination stock custody: `restock_gap_at_destination`, `MoveCargo` search transitions, and `store_stock` affordance targeting must agree on what counts as delivered stock.

## Architecture Check

1. Conditional terminal logic based on destination characteristics is cleaner than separate goal kinds. `MoveCargo` should remain one goal kind, but destination facilities with `StockStoragePolicy` must count only facility-custody stock (stored/displayed), never merely carried stock at the destination place.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Facility restock: facility-custody stock satisfies `MoveCargo`, but carried-at-destination stock does not → goal-model + search focused tests
2. Facility `MoveCargo` plan shape includes explicit storage after travel when a controlled `StockStoragePolicy` facility exists → search focused test
3. `store_stock` affordance can target a directly possessed lot so the lawful transport path is searchable → focused affordance/authoritative test
4. Non-facility MoveCargo unchanged → existing behavior preserved (regression test)

## What to Change

### 1. Evolve MoveCargo terminal condition

In `goal_model.rs` / enterprise helpers: when the destination place contains a controlled facility with `StockStoragePolicy`, destination stock for `MoveCargo` must be counted from facility custody state, not from all controlled stock at the place. Carried stock at the destination place is insufficient until it has been stored. Without `StockStoragePolicy`, the existing non-facility behavior remains.

### 2. Update planner transitions and relevant ops

In planner/search modules: facility-targeted `MoveCargo` must be able to continue past travel into `store_stock`, with a hypothetical transition that places the lot into the facility stock container. `MoveCargo` should treat `StockManagement` as relevant when the facility path exists.

### 3. Align authoritative affordance targeting for store_stock

In authoritative action registration, `store_stock` must target directly possessed lots so the planner can lawfully discover the post-travel storage step for carried cargo.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/enterprise.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/search/` (modify — terminal condition logic)
- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-systems/src/stock_actions.rs` (modify)

## Out of Scope

- AI planning for staging workflow (007)
- Non-merchant MoveCargo behavior unchanged — no modifications
- Theft distinction (008)
- Golden tests (010)

## Acceptance Criteria

### Tests That Must Pass

1. Facility restock requires stock in container, not mere arrival
2. Mere arrival at facility destination is insufficient terminal
3. Facility `MoveCargo` can search through `store_stock` after travel
4. `store_stock` affordance targets carried lots lawfully
5. Non-facility MoveCargo behavior unchanged
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. MoveCargo for non-facility destinations is unaffected
2. Plan search correctly distinguishes facility vs non-facility terminals
3. Belief-only planning — terminal checks use belief-accessible facility structure, not omniscient state
4. Carried stock, stored stock, and displayed stock remain distinct custody states

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` / `enterprise.rs` — facility restock excludes carried-at-destination stock
2. `crates/worldwake-ai/src/search/` — facility plan shape includes `store_stock`
3. `crates/worldwake-ai/src/planner_ops.rs` — `store_stock` hypothetical transition places lots into stock container
4. `crates/worldwake-systems/src/stock_actions.rs` — `store_stock` targets directly possessed lots
5. `crates/worldwake-ai/src/goal_model.rs` — non-facility MoveCargo unchanged

### Commands

1. `cargo test -p worldwake-ai -- move_cargo`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome (2026-04-01)

### What changed

1. `MoveCargo` merchant restock terminals now distinguish facility destinations from ordinary destinations. For destinations with a controlled `StockStoragePolicy`, `MoveCargo` only completes when the stock is in facility custody rather than merely carried at the destination place.
2. Planning snapshots and planning state now preserve belief-accessible `StockStoragePolicy` data so hypothetical search can reason about lawful facility storage without reading omniscient world state.
3. `MoveCargo` now treats `StockManagement` as a relevant operator family, and the hypothetical planner transition for `store_stock` places a carried lot into the controlled facility stock container.
4. `store_stock` authoritative targeting now uses directly possessed lots, aligning the searchable affordance path with the real action contract.
5. Focused AI tests now prove both the stronger facility-custody terminal and the `pick_up -> travel -> store_stock` facility restock plan shape.

### Deviations from original plan

1. `crates/worldwake-ai/src/candidate_generation.rs` did not require code changes. The necessary behavior emerged from correcting the destination custody terminal, exposing storage policy through the belief/snapshot layer, and giving search a lawful `store_stock` continuation.

### Verification

- `cargo test -p worldwake-ai move_cargo -- --nocapture`
- `cargo test -p worldwake-ai cargo_search -- --nocapture`
- `cargo test -p worldwake-ai authoritative_partial_cargo_pickup_can_reach_goal_satisfaction -- --nocapture`
- `cargo test -p worldwake-systems stock_actions -- --nocapture`
- `cargo clippy --workspace --all-targets -- -D warnings`
