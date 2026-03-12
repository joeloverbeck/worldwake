# HARCARGOACON-004: Emit MoveCargo candidates from local controllable cargo

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai (candidate_generation module)
**Deps**: HARCARGOACON-001 (new MoveCargo variant), HARCARGOACON-002 (BeliefView helpers), HARCARGOACON-003 (restock_gap_at_destination)

## Problem

`generate_candidates()` never emits `MoveCargo` goals. The `deferred_goal_kinds_are_not_emitted` test at line 1542 explicitly asserts `MoveCargo` is excluded. Cargo movement cannot function as an autonomous goal until candidates are actually generated from concrete local cargo and destination demand.

## Assumption Reassessment (2026-03-12)

1. `generate_candidates()` currently never emits `MoveCargo` — confirmed via test at `candidate_generation.rs:1542`
2. `MerchandiseProfile.home_market` provides destination — confirmed in `worldwake-core`
3. `DemandMemory` provides demand observations per place/commodity — confirmed
4. `restock_gap_at_destination` will exist after HARCARGOACON-003 — per dependency
5. `local_controlled_lots_for` and `controlled_commodity_quantity_at_place` will exist after HARCARGOACON-002 — per dependency
6. The spec requires `deliverable_quantity` as a private helper, not on `BeliefView` — confirmed per spec Section C.3

## Architecture Check

1. Cargo candidates are derived from concrete local state (possessions + ground lots at current place) — no omniscient remote queries
2. `deliverable_quantity` is a private candidate_generation helper, not a trait method — keeps planning concerns out of the belief surface
3. Evidence model stores `deliverable_quantity` for planner batch sizing without coupling it to goal identity

## What to Change

### 1. Add cargo candidate derivation function

Add a private function in `candidate_generation.rs` that:
1. Gets agent's `MerchandiseProfile` → `home_market` destination
2. Gets agent's current `effective_place`
3. If agent is already at `home_market`, skip (no delivery needed)
4. For each commodity in agent's `DemandMemory` at `home_market`:
   a. Call `restock_gap_at_destination(view, agent, home_market, commodity)` — if `None`, skip
   b. Call `local_controlled_lots_for(view, agent, current_place, commodity)` — if empty, skip
   c. Compute `deliverable_quantity` = `min(local_quantity, restock_gap, carry_fit)`
   d. If `deliverable_quantity == Quantity(0)`, skip
   e. Emit `GroundedGoal` with key `GoalKind::MoveCargo { commodity, destination: home_market }`

### 2. `deliverable_quantity` private helper

```rust
fn deliverable_quantity(
    view: &dyn BeliefView,
    agent: EntityId,
    current_place: EntityId,
    destination: EntityId,
    commodity: CommodityKind,
) -> Quantity { ... }
```

Uses:
- `controlled_commodity_quantity_at_place(agent, current_place, commodity)` for local stock
- `restock_gap_at_destination(view, agent, destination, commodity)` for gap
- carry capacity / load math for carry fit
- Returns `min(local, gap, carry_fit)`

### 3. Wire into `generate_candidates()`

Call the new cargo candidate function from the main `generate_candidates()` orchestrator.

### 4. Update `deferred_goal_kinds_are_not_emitted` test

Remove `GoalKind::MoveCargo { .. }` from the assertion at line 1562. `MoveCargo` is no longer deferred. Update the test name if `MoveCargo` was the primary motivation (keep `SellCommodity` and `BuryCorpse` assertions).

### 5. Evidence model for GroundedGoal

The `GroundedGoal` evidence for cargo must include:
- The destination place (already in `GoalKind`)
- The concrete local lot entity IDs that could satisfy the batch (in `targets` or evidence field)
- The computed `deliverable_quantity` (for planner batch sizing)

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add cargo candidate logic, update test)

## Out of Scope

- Modifying `BeliefView` trait (done in HARCARGOACON-002)
- Modifying `enterprise.rs` (done in HARCARGOACON-003)
- Changing goal satisfaction semantics (HARCARGOACON-005)
- Removing `MoveCargo` from `unsupported_goal()` in `search.rs` (HARCARGOACON-005)
- Adding non-merchant cargo delivery (e.g., generic hauling jobs)
- Emitting cargo goals from remote stock the agent cannot act on locally

## Acceptance Criteria

### Tests That Must Pass

1. New test: `MoveCargo` candidate emitted when agent has local controllable commodity and `restock_gap_at_destination` returns a gap
2. New test: No `MoveCargo` candidate emitted when agent has no local stock of demanded commodity
3. New test: No `MoveCargo` candidate emitted when agent is already at home_market
4. New test: No `MoveCargo` candidate emitted from remote stock the agent is not positioned to move (locality)
5. New test: `deliverable_quantity` is capped by carry capacity
6. New test: No `MoveCargo` emitted when `deliverable_quantity` is zero (full carry or zero gap)
7. Updated test: `deferred_goal_kinds_are_not_emitted` no longer asserts `MoveCargo` exclusion
8. `cargo test --workspace` and `cargo clippy --workspace` pass

### Invariants

1. Cargo candidates only derived from local controllable cargo (direct possessions or local ground lots)
2. `deliverable_quantity` uses `Quantity`, never floats
3. Goal identity is `MoveCargo { commodity, destination }` — no lot entity or quantity in the key
4. No `MoveCargo` emitted when `deliverable_quantity == Quantity(0)`
5. Candidate generation remains deterministic

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — `cargo_candidate_emitted_from_local_stock_and_demand`, `no_cargo_candidate_without_local_stock`, `no_cargo_candidate_when_at_destination`, `no_cargo_candidate_from_remote_stock`, `deliverable_quantity_capped_by_carry`, `no_cargo_when_zero_deliverable`
2. `crates/worldwake-ai/src/candidate_generation.rs` — update `deferred_goal_kinds_are_not_emitted`

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
