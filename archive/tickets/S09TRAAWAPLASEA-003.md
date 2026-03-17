# S09TRAAWAPLASEA-003: Add A* heuristic to plan search ordering

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — modified search node ordering in `search.rs`
**Deps**: S09TRAAWAPLASEA-001 (distance matrix), S09TRAAWAPLASEA-002 (goal-relevant places)

## Problem

The GOAP plan search orders frontier nodes by g-cost only (`total_estimated_ticks`), making it a uniform-cost/Dijkstra search. At hub nodes with 7+ outgoing travel edges, this causes combinatorial explosion — the search expands all travel directions equally. This ticket adds an A* heuristic (h = minimum travel distance to nearest goal-relevant place) so the search prioritizes directions toward the goal.

## Assumption Reassessment (2026-03-17, corrected)

1. `SearchNode` struct at `search.rs:16-20` has fields `state: PlanningState<'snapshot>`, `steps: Vec<PlannedStep>`, `total_estimated_ticks: u32` — confirmed. No `heuristic_ticks` field exists yet.
2. `compare_search_nodes` at `search.rs:466-471` orders by `total_estimated_ticks` then `steps.len()` then `steps` — confirmed. Pure g-cost ordering.
3. `search_plan` signature at `search.rs:97-104` takes `snapshot`, `goal`, `semantics_table`, `registry`, `handlers`, `budget` — confirmed. Does not currently receive goal-relevant places.
4. `PlanningSnapshot::min_travel_ticks_to_any` exists (ticket 001 complete) — confirmed at `planning_snapshot.rs:215`.
5. `GoalKindPlannerExt::goal_relevant_places` exists (ticket 002 complete) — confirmed at `goal_model.rs:65`. **Note**: signature is `fn goal_relevant_places(&self, state: &PlanningState<'_>, recipes: &RecipeRegistry) -> Vec<EntityId>` — takes `&RecipeRegistry` as an additional parameter.
6. Successor nodes are built in `build_successor` at `search.rs:186` within the search loop — confirmed.
7. The `GroundedGoal` struct wraps a `GoalKind` — confirmed, accessible via `goal.key.kind`.
8. `build_candidate_plans` at `agent_tick.rs:878` calls `search_plan` — `ctx.recipe_registry` is available via `AgentTickContext`.
9. `PlanningState::effective_place_ref(PlanningEntityRef::Authoritative(actor))` resolves actor place — confirmed at `planning_state.rs:216`.

## Architecture Check

1. A* with admissible heuristic (shortest-path distance never overestimates) preserves optimality — the search still finds the lowest-cost plan.
2. The heuristic is consistent (h(n) ≤ cost(n,n') + h(n')) because travel costs match distance matrix entries exactly.
3. Tie-breaking on g-cost when f-costs are equal (prefer less committed cost) is a standard A* optimization.

## What to Change

### 1. Add `heuristic_ticks: u32` field to `SearchNode`

New field on the `SearchNode` struct. Initialized to 0 for the root node if the actor is already at a goal-relevant place, or to the minimum travel distance otherwise.

### 2. Accept precomputed goal-relevant places as a parameter

Add `goal_relevant_places: &[EntityId]` to `search_plan`'s signature. Callers (in `agent_tick.rs`) compute the places before calling search via `goal.kind.goal_relevant_places(&PlanningState::new(&snapshot), recipes)`. This keeps the search algorithm domain-agnostic — it receives a spatial hint without needing to know about recipes, workstations, or production (Principle 24: systems interact through state, not through each other).

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

- `crates/worldwake-ai/src/search.rs` (modify — add heuristic field, add `goal_relevant_places` param, modify comparator, compute heuristic in expansion)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — thread `recipe_registry` into `build_candidate_plans`, compute goal-relevant places at call site before `search_plan`)

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

## Outcome

**Completion date**: 2026-03-17

**What changed**:
- `search.rs`: Added `heuristic_ticks: u32` to `SearchNode`, `compute_heuristic()` helper, `goal_relevant_places: &[EntityId]` param to `search_plan`, `root_node`, and `build_successor`. Updated `compare_search_nodes` to order by f = g + h with g-cost tie-breaking.
- `agent_tick.rs`: Threaded `recipe_registry` through `build_candidate_plans`, `plan_and_validate_next_step`, and `plan_and_validate_next_step_traced`. Goal-relevant places precomputed at call site before `search_plan`.
- 7 new unit tests: heuristic at goal place (0), heuristic two hops away (8), nearest of multiple (3), empty places (0), f-cost ordering, g-cost tie-breaking, uniform-cost degradation.

**Deviations from original plan**:
- Ticket originally called `goal_relevant_places` inside `search_plan`. Corrected to precompute at caller and pass `&[EntityId]` into search, per Principle 24 (systems interact through state, not through each other). `search_plan` stays domain-agnostic.
- `goal_relevant_places` takes `&RecipeRegistry` (not documented in original ticket). Threading `recipe_registry` through the agent_tick call chain was required.

**Verification**:
- `cargo test --workspace` — all pass (473 AI tests, 0 failures, 2 ignored supply chain tests per ticket 005)
- `cargo clippy --workspace` — clean
- All existing golden tests pass unchanged (heuristic is 0 with empty goal_relevant_places)
