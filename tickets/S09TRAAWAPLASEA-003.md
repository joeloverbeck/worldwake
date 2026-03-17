# S09TRAAWAPLASEA-003: Add A* heuristic to plan search ordering

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — modified search node ordering in `search.rs`
**Deps**: S09TRAAWAPLASEA-001 (distance matrix), S09TRAAWAPLASEA-002 (goal-relevant places)

## Problem

The GOAP plan search orders frontier nodes by g-cost only (`total_estimated_ticks`), making it a uniform-cost/Dijkstra search. At hub nodes with 7+ outgoing travel edges, this causes combinatorial explosion — the search expands all travel directions equally. This ticket adds an A* heuristic (h = minimum travel distance to nearest goal-relevant place) so the search prioritizes directions toward the goal.

## Assumption Reassessment (2026-03-17)

1. `SearchNode` struct at `search.rs:15-20` has fields `state: PlanningState<'snapshot>`, `steps: Vec<PlannedStep>`, `total_estimated_ticks: u32` — confirmed. No `heuristic_ticks` field exists yet.
2. `compare_search_nodes` at `search.rs:466-471` orders by `total_estimated_ticks` then `steps.len()` then `steps` — confirmed. Pure g-cost ordering.
3. `search_plan` signature at `search.rs:97-104` takes `snapshot`, `goal`, `semantics_table`, `registry`, `handlers`, `budget` — confirmed. Does not currently receive goal-relevant places.
4. `PlanningSnapshot::min_travel_ticks_to_any` will exist after ticket 001 — dependency.
5. `GoalKindPlannerExt::goal_relevant_places` will exist after ticket 002 — dependency.
6. Successor nodes are built in `build_successor` (or equivalent expansion logic) within the search loop — confirmed.
7. The `GroundedGoal` struct wraps a `GoalKind` — confirmed, accessible via `goal.kind`.

## Architecture Check

1. A* with admissible heuristic (shortest-path distance never overestimates) preserves optimality — the search still finds the lowest-cost plan.
2. The heuristic is consistent (h(n) ≤ cost(n,n') + h(n')) because travel costs match distance matrix entries exactly.
3. Tie-breaking on g-cost when f-costs are equal (prefer less committed cost) is a standard A* optimization.

## What to Change

### 1. Add `heuristic_ticks: u32` field to `SearchNode`

New field on the `SearchNode` struct. Initialized to 0 for the root node if the actor is already at a goal-relevant place, or to the minimum travel distance otherwise.

### 2. Compute goal-relevant places once at search start

At the top of `search_plan`, call `goal.kind.goal_relevant_places(&initial_state)` to get the list of goal-relevant places. Store as a local `Vec<EntityId>`.

### 3. Compute heuristic in successor construction

When building each successor node, compute:
```rust
let actor_place = /* resolve actor's simulated place from successor state */;
let heuristic_ticks = snapshot
    .min_travel_ticks_to_any(actor_place, &goal_relevant_places)
    .unwrap_or(0);
```
Store in the successor's `heuristic_ticks` field.

### 4. Modify `compare_search_nodes` to use f = g + h

```rust
fn compare_search_nodes(left: &SearchNode<'_>, right: &SearchNode<'_>) -> Ordering {
    let left_f = left.total_estimated_ticks.saturating_add(left.heuristic_ticks);
    let right_f = right.total_estimated_ticks.saturating_add(right.heuristic_ticks);
    left_f.cmp(&right_f)
        .then_with(|| left.total_estimated_ticks.cmp(&right.total_estimated_ticks))
        .then_with(|| left.steps.len().cmp(&right.steps.len()))
        .then_with(|| left.steps.cmp(&right.steps))
}
```

### 5. Resolve actor place from PlanningState

Since `PlanningState` has no `actor_place()` convenience method, use the existing pattern:
```rust
let actor = state.snapshot().actor();
let actor_place = state.effective_place_ref(PlanningEntityRef::Authoritative(actor));
```
Extract this into a small helper if it's used in multiple places within `search.rs`.

## Files to Touch

- `crates/worldwake-ai/src/search.rs` (modify — add heuristic field, modify comparator, compute heuristic in expansion)

## Out of Scope

- Distance matrix computation (ticket 001 — must be complete)
- Goal-relevant places implementation (ticket 002 — must be complete)
- Travel pruning (ticket 004)
- Golden test changes (ticket 005)
- Modifying `PlanningSnapshot`, `PlanningState`, or `GoalKindPlannerExt`
- Changing `PlanSearchResult` or `PlannedPlan` structures
- Adding heuristic values to `DecisionTrace` or `PlannedStepSummary` (optional future diagnostic work)

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: Root `SearchNode` at a goal-relevant place has `heuristic_ticks == 0`.
2. Unit test: Root `SearchNode` two hops from goal-relevant place has `heuristic_ticks` equal to the shortest-path distance.
3. Unit test: `compare_search_nodes` with equal g-cost but different h-cost orders by f = g + h.
4. Unit test: `compare_search_nodes` with equal f-cost prefers lower g-cost (tie-breaking).
5. All existing golden tests pass: `cargo test -p worldwake-ai` — no regressions.
6. Existing golden tests should use the same or fewer node expansions (verifiable via decision traces if enabled).
7. `cargo clippy --workspace`

### Invariants

1. Heuristic is admissible: `heuristic_ticks` never exceeds actual travel cost to the nearest goal-relevant place.
2. When `goal_relevant_places` is empty, `heuristic_ticks` is always 0 — search degrades to uniform-cost (no regression).
3. Determinism is preserved — `BTreeMap`-based distance matrix and deterministic place ordering ensure identical search behavior.
4. `PlanSearchResult` variants are unchanged — callers are unaffected.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` — unit tests for heuristic computation, comparator behavior with heuristic, regression tests confirming existing plans are still found

### Commands

1. `cargo test -p worldwake-ai search`
2. `cargo test -p worldwake-ai` (all golden tests)
3. `cargo test --workspace && cargo clippy --workspace`
