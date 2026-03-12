# HARCARGOACON-002: Add destination-aware read helpers to BeliefView and PlanningState

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-sim (BeliefView trait), worldwake-ai (PlanningState, PlanningSnapshot)
**Deps**: HARCARGOACON-001 (new MoveCargo variant must exist)

## Problem

Cargo satisfaction and candidate generation need destination-aware commodity queries: "how much of commodity X does agent A control at place P?" The existing `commodity_quantity(holder, kind)` sums across all places, which is insufficient for destination-specific cargo logic.

## Assumption Reassessment (2026-03-12)

1. `BeliefView` trait is at `belief_view.rs:10` with ~50 methods — confirmed
2. `commodity_quantity(holder, kind)` exists but is not place-filtered — confirmed at `belief_view.rs:20`
3. `OmniscientBeliefView` implements `BeliefView` — confirmed at `omniscient_belief_view.rs:126`
4. `PlanningState` has `commodity_quantity_ref()` at `planning_state.rs:187` — confirmed
5. All test `BeliefView` impls across AI crate need updating when trait changes — confirmed (8+ impl blocks)

## Architecture Check

1. Adding two new trait methods is cleaner than overloading existing `commodity_quantity` with optional place parameter
2. Deterministic ordering via `BTreeMap`/sorted `Vec` maintained in all implementations

## What to Change

### 1. `BeliefView` trait — add two methods

```rust
fn controlled_commodity_quantity_at_place(
    &self,
    agent: EntityId,
    place: EntityId,
    commodity: CommodityKind,
) -> Quantity;

fn local_controlled_lots_for(
    &self,
    agent: EntityId,
    place: EntityId,
    commodity: CommodityKind,
) -> Vec<EntityId>;
```

### 2. `OmniscientBeliefView` — implement both methods

- `controlled_commodity_quantity_at_place`: filter `direct_possessions(agent)` to lots of the given commodity that are effectively at the given place, sum quantities
- `local_controlled_lots_for`: filter `direct_possessions(agent)` to lots of the given commodity at the given place, return entity IDs in deterministic order

### 3. `PlanningState` — implement both methods

Mirror the authoritative implementations but using `PlanningEntityRef`-aware lookups and respecting hypothetical entity overrides/shadows.

### 4. All test `BeliefView` impls — add stub implementations

Every `impl BeliefView for TestBeliefView` (and similar) across the AI crate must implement the two new methods. Default stubs returning `Quantity(0)` / `Vec::new()` are acceptable for tests that don't exercise cargo logic.

Files with test `BeliefView` impls:
- `crates/worldwake-ai/src/pressure.rs`
- `crates/worldwake-ai/src/planning_snapshot.rs`
- `crates/worldwake-ai/src/failure_handling.rs`
- `crates/worldwake-ai/src/plan_revalidation.rs`
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/planner_ops.rs`
- `crates/worldwake-ai/src/ranking.rs`
- `crates/worldwake-ai/src/search.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-sim/src/trade_valuation.rs`
- `crates/worldwake-sim/src/affordance_query.rs`

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait definition)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (modify — authoritative impl)
- `crates/worldwake-ai/src/planning_state.rs` (modify — planning impl)
- `crates/worldwake-ai/src/pressure.rs` (modify — test stub)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — test stub)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — test stub)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — test stub)
- `crates/worldwake-ai/src/goal_model.rs` (modify — test stub)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — test stub)
- `crates/worldwake-ai/src/ranking.rs` (modify — test stub)
- `crates/worldwake-ai/src/search.rs` (modify — test stub)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — test stub)
- `crates/worldwake-sim/src/trade_valuation.rs` (modify — test stub)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — test stub)

## Out of Scope

- Using these helpers in candidate generation (HARCARGOACON-004)
- Using these helpers in goal satisfaction (HARCARGOACON-005)
- Adding `restock_gap_at_destination` (HARCARGOACON-003)
- Modifying existing `commodity_quantity` behavior
- Any changes to `GoalKind` or `GoalKey` (done in HARCARGOACON-001)

## Acceptance Criteria

### Tests That Must Pass

1. New unit test in `omniscient_belief_view.rs`: `controlled_commodity_quantity_at_place` returns correct quantity for agent possessions at a specific place, zero for possessions at other places
2. New unit test in `omniscient_belief_view.rs`: `local_controlled_lots_for` returns only lots at the specified place in deterministic order
3. New unit test in `planning_state.rs`: both methods work correctly with hypothetical entity overrides
4. `cargo test --workspace` — all existing tests pass with new trait methods
5. `cargo clippy --workspace` — no warnings

### Invariants

1. `controlled_commodity_quantity_at_place` returns `Quantity(0)` when no matching lots exist at the place
2. `local_controlled_lots_for` returns entities in deterministic (sorted) order
3. Both methods only count lots the agent directly possesses (not remote lots, not other agents' lots)
4. Both methods are consistent: `controlled_commodity_quantity_at_place` equals the sum of quantities of entities returned by `local_controlled_lots_for`
5. `PlanningState` implementations respect hypothetical entity shadows and overrides

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/omniscient_belief_view.rs` — `controlled_commodity_quantity_at_place_filters_by_place`, `local_controlled_lots_for_returns_deterministic_order`
2. `crates/worldwake-ai/src/planning_state.rs` — `controlled_commodity_quantity_at_place_with_hypotheticals`

### Commands

1. `cargo test -p worldwake-sim omniscient_belief_view`
2. `cargo test -p worldwake-ai planning_state`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
