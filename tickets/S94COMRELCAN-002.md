# S94COMRELCAN-002: Implement commodity-relevance candidate filter

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — planner-internal filter only
**Deps**: archive/tickets/S94COMRELCAN-001.md

## Problem

The GOAP planner generates 1400-5700 candidates for commodity goals because the candidate pipeline filters by operation kind but not by commodity relevance. `AcquireCommodity(Water)` generates `MoveCargo(Waste)`, `Trade(Guard)`, `QueueForFacility(Mill)`, etc. — all irrelevant to water acquisition. This ticket adds a post-pipeline filter that retains only candidates whose targets relate to the goal's target commodity.

## Assumption Reassessment (2026-04-11)

1. `search_candidates()` at `crates/worldwake-ai/src/search/candidates.rs:101` returns `Vec<SearchCandidate>` after applying binding, blocked-facility, availability, and place-blocker filters. It has 11 parameters — the spec chose to add the new filter at the call site rather than widening this function.
2. `search_plan_with_trace_metadata()` at `crates/worldwake-ai/src/search/mod.rs:135` receives `recipes: &RecipeRegistry` as a parameter. The integration point is between the `search_candidates()` return (line ~363) and the tactical filter application (line ~374).
3. `PlannerOpSemantics` at `crates/worldwake-ai/src/planner_ops.rs:57` provides `op_kind: PlannerOpKind` for classifying candidates by operation type.
4. Belief view methods confirmed: `item_lot_commodity()` on `InventoryBeliefView` (planning_state.rs:2049), `resource_source()` on `FacilityBeliefView` (planning_state.rs:2111).
5. `SearchCandidate` struct at `candidates.rs:21` has `def_id: ActionDefId` and `targets: Vec<EntityId>` — the filter uses `def_id` to look up `PlannerOpKind` via `semantics_table`, and `targets` to resolve commodity kind.
6. Trade payloads: `SearchCandidate` has `payload_override: Option<ActionPayload>` — trade candidates carry payload with commodity field.
7. Craft payloads carry `recipe_id` in the payload — used for recipe lookup via `RecipeRegistry`.
8. The filter is a heuristic removal of provably-irrelevant candidates (not a heuristic addition). Per precision rule 12: the missing substrate is "commodity-relevance awareness in candidate generation." This ticket introduces that substrate. No unrelated regressions are opened because the filter only prunes candidates with positively-resolved non-matching commodities; unknown/unresolvable commodities pass (conservative default).

## Architecture Check

1. Placing the filter between `search_candidates()` and the tactical filter maintains clean separation: root filtering (binding, availability, place) → commodity relevance → tactical scoping (location). Each layer has a distinct concern.
2. No backward-compatibility shims. The filter applies unconditionally when the goal has a target commodity. No opt-out path or feature flag.

## Verification Layers

1. Commodity-irrelevant candidates are pruned → decision trace shows `CommodityIrrelevant` entries for filtered candidates
2. Commodity-relevant candidates are retained → focused unit tests with known candidate sets
3. Travel candidates always pass → focused unit test
4. Unknown/unresolvable commodity candidates pass (conservative default) → focused unit test
5. Non-commodity goals bypass the filter entirely → focused unit test with `Sleep` or similar goal
6. Single-layer ticket (planner-internal) — no authoritative state changes

## What to Change

### 1. Add `apply_commodity_relevance_filter()` function

In `crates/worldwake-ai/src/search/candidates.rs`:

Add a `pub(super)` function:

```rust
pub(super) fn apply_commodity_relevance_filter(
    candidates: &mut Vec<SearchCandidate>,
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    recipes: &RecipeRegistry,
    root_candidates: Option<&mut Vec<crate::decision_trace::RootCandidateTrace>>,
)
```

Filter logic per `PlannerOpKind`:
- **Travel**: Always pass
- **MoveCargo**: Pass if `state.item_lot_commodity(target)` == goal commodity
- **Trade**: Pass if payload's commodity == goal commodity
- **QueueForFacilityUse**: Pass if `state.resource_source(target).commodity` == goal commodity
- **Harvest**: Pass if `state.resource_source(target).commodity` == goal commodity
- **Craft**: Pass if recipe (from payload's `recipe_id`) has goal commodity as input or output
- **Heal, AskWitness, all others**: Always pass

Bypass: If `goal.key.kind.target_commodity(recipes)` returns `None`, return immediately without filtering.

Conservative default: If commodity kind cannot be positively resolved (method returns `None` or entity not in beliefs), the candidate passes.

Record filtered candidates in `root_candidates` trace with `RootCandidateFilterReason::CommodityIrrelevant`.

### 2. Integrate filter into search pipeline

In `crates/worldwake-ai/src/search/mod.rs`, in `search_plan_with_trace_metadata()`:

After the `search_candidates()` call returns and before the tactical candidate filter runs, call `apply_commodity_relevance_filter()` on the returned candidate vec. Pass `recipes` (already available at line 135), `goal`, `&node.state`, `semantics_table`, and the trace sink.

## Files to Touch

- `crates/worldwake-ai/src/search/candidates.rs` (modify) — new filter function
- `crates/worldwake-ai/src/search/mod.rs` (modify) — integration call site

## Out of Scope

- Modifying `matches_binding()` — operates at binding level, not commodity relevance
- Modifying the tactical candidate filter — location-based scoping is orthogonal
- Modifying `search_candidates()`'s parameter list
- Per-agent filter tuning via `CognitiveProfile`
- Golden test rewrites (ticket 003)
- Changing `CognitiveProfile` or `ExecutionBudget` parameters
- Addressing non-commodity goals (Sleep, Combat, etc.)

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit tests for `apply_commodity_relevance_filter()` covering each `PlannerOpKind` filter rule
2. New focused unit test for conservative default (unknown commodity passes)
3. New focused unit test for bypass condition (non-commodity goal)
4. Existing suite: `cargo test --workspace`

### Invariants

1. All commodity-relevant candidates are retained — the filter never removes a candidate whose commodity matches the goal
2. Travel candidates always pass regardless of goal commodity
3. Unknown/unresolvable commodity kinds pass the filter (conservative default — no false negatives)
4. Non-commodity goals bypass the filter entirely (no candidates removed)
5. Filtered candidates appear in decision traces with `CommodityIrrelevant` reason and both the candidate's and goal's commodity kinds

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/candidates.rs` (new `#[cfg(test)]` tests) — focused unit tests for the filter function covering: each PlannerOpKind rule, conservative default for unresolvable commodities, bypass for non-commodity goals, correct trace recording
2. `crates/worldwake-ai/src/search/tests.rs` (new tests) — integration-level tests using `search_plan_with_trace_metadata()` to verify the filter is called in the pipeline and produces expected candidate count reductions

### Commands

1. `cargo test -p worldwake-ai commodity_relevance` — new focused tests
2. `cargo test -p worldwake-ai search` — search module integration
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean
4. `cargo test --workspace` — full regression
