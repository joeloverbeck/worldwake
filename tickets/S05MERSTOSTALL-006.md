# S05MERSTOSTALL-006: Evolve MoveCargo for merchant restock to target facility storage

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — MoveCargo terminal condition and evidence changes
**Deps**: S05MERSTOSTALL-005

## Problem

Merchant restock via `MoveCargo` should target the facility's `stock_container`, not mere arrival-with-possession. When a destination has a `StockStoragePolicy`, the terminal condition must be "stock is in the facility's containers," not "carrier arrived at the place."

## Assumption Reassessment (2026-04-01)

1. `MoveCargo` goal/action exists in the planner — check exact terminal condition and evidence generation in `goal_model.rs` and related files.
2. `StockStoragePolicy` is queryable on facility entities — confirmed via S05MERSTOSTALL-001 outcome.
3. Sale visibility evolution (005) is complete — facility-based model is the active paradigm.
4. Non-merchant `MoveCargo` (destinations without `StockStoragePolicy`) must remain unchanged — behavioral branch, not replacement.
5. Evidence generation for plan search must reflect the new terminal — check `search/` modules.

## Architecture Check

1. Conditional terminal logic based on destination characteristics is cleaner than separate goal kinds. `MoveCargo` checks for `StockStoragePolicy` at destination and adjusts its terminal condition accordingly — single goal kind, context-dependent completion.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Facility restock: stock in container is terminal → plan search terminal condition (focused test)
2. Mere arrival insufficient for facility restock → plan search rejects early termination (focused test)
3. Non-facility MoveCargo unchanged → existing behavior preserved (regression test)
4. Evidence generation reflects new terminal → planner evidence (focused test)

## What to Change

### 1. Evolve MoveCargo terminal condition

In `goal_model.rs` (or equivalent): when the destination entity has `StockStoragePolicy`, terminal condition = stock lot exists in facility's `stock_container`. Without `StockStoragePolicy`, existing arrival-based terminal remains.

### 2. Update evidence generation

In planner search modules: evidence for facility-targeted MoveCargo must include stock container placement, not just carrier location.

### 3. Update candidate generation

In `candidate_generation.rs`: ensure MoveCargo candidates for facility destinations generate the correct plan structure (arrival + store_stock).

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/search/` (modify — terminal condition logic)

## Out of Scope

- AI planning for staging workflow (007)
- Non-merchant MoveCargo behavior unchanged — no modifications
- Theft distinction (008)
- Golden tests (010)

## Acceptance Criteria

### Tests That Must Pass

1. Facility restock requires stock in container, not mere arrival
2. Mere arrival at facility destination is insufficient terminal
3. Non-facility MoveCargo behavior unchanged
4. Evidence generation reflects container placement for facility destinations
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. MoveCargo for non-facility destinations is unaffected
2. Plan search correctly distinguishes facility vs non-facility terminals
3. Belief-only planning — terminal checks use belief state, not world state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — facility terminal requires container placement
2. `crates/worldwake-ai/src/search/` — mere arrival rejected for facility restock
3. `crates/worldwake-ai/src/goal_model.rs` — non-facility MoveCargo unchanged

### Commands

1. `cargo test -p worldwake-ai -- move_cargo`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
