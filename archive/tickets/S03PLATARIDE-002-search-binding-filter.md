# S03PLATARIDE-002: Wire binding filter into `search_candidates()`

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `search_candidates()` gains a `.retain()` call after the facility-use blocked filter
**Deps**: S03PLATARIDE-001 (needs `matches_binding()` on `GoalKindPlannerExt`)

## Problem

Even after `matches_binding()` exists on `GoalKindPlannerExt`, search will not use it until the candidate list in `search_candidates()` is filtered through binding before successor construction. Without this filter, wrong-target affordances are explored, wasting budget and potentially producing incorrect plans.

## Assumption Reassessment (2026-03-17)

1. `search_candidates()` is defined in `crates/worldwake-ai/src/search.rs:372` and returns a `Vec<SearchCandidate>`.
2. The existing facility-use blocked filter is at search.rs:398: `candidates.retain(|candidate| !candidate_uses_blocked_facility_use(candidate, &node.state, registry));`
3. `search_candidates` already receives `goal: &GroundedGoal` (search.rs:373) and `semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>` (search.rs:375).
4. `SearchCandidate` has `def_id: ActionDefId` and `authoritative_targets: Vec<EntityId>` fields (search.rs:32–34).
5. `GroundedGoal` contains `key: GoalKey` which contains `kind: GoalKind`.
6. `PlannerOpSemantics` has `op_kind: PlannerOpKind` field (planner_ops.rs:36).

## Architecture Check

1. Placing the `.retain()` call immediately after the existing facility-use filter keeps all candidate filtering in one location, before successor construction.
2. No new function signature changes or API breaks — this is an internal filter addition.
3. No backward-compatibility shims.

## What to Change

### 1. Add binding `.retain()` in `search_candidates()`

In `crates/worldwake-ai/src/search.rs`, immediately after the existing line:
```rust
candidates.retain(|candidate| !candidate_uses_blocked_facility_use(candidate, &node.state, registry));
```

Add:
```rust
// Reject candidates whose authoritative targets violate goal binding.
candidates.retain(|candidate| {
    let Some(semantics) = semantics_table.get(&candidate.def_id) else {
        return true;
    };
    goal.key.kind.matches_binding(&candidate.authoritative_targets, semantics.op_kind)
});
```

This is exactly 5 lines of new code. The `matches_binding` call handles all dispatch logic (auxiliary bypass, empty targets bypass, flexible goals, exact matching).

## Files to Touch

- `crates/worldwake-ai/src/search.rs` (modify — add `.retain()` call after line 398)

## Out of Scope

- The `matches_binding()` implementation itself — that is S03PLATARIDE-001.
- `BindingRejection` trace recording — that is S03PLATARIDE-003 (the trace captures rejections from this filter, but the filter itself just discards candidates silently; trace integration comes later).
- Any changes to affordance enumeration in `worldwake-sim`.
- Any changes to `search_plan()` signature or `PlanSearchResult` variants.
- Any changes to candidate generation or ranking.
- Golden integration tests proving multi-entity scenarios — that is S03PLATARIDE-004.

## Acceptance Criteria

### Tests That Must Pass

1. All existing `cargo test -p worldwake-ai` tests must continue to pass (the filter is a no-op for flexible goals and correct-target scenarios).
2. All existing golden tests must pass (agents with exact-bound goals already had correct targets in their affordances; the filter only rejects wrong targets).
3. `cargo clippy --workspace` clean.

### Invariants

1. Wrong-target affordances are discarded **before** successor construction, not explored and rejected later.
2. Candidates with empty `authoritative_targets` (planner-only synthetic candidates) bypass binding — handled by `matches_binding()` returning `true`.
3. Auxiliary ops (Travel, Trade, etc.) are never rejected by the binding filter.
4. Planner determinism preserved — `.retain()` is deterministic given deterministic input ordering.
5. Affordance enumeration remains unchanged — filtering happens only within search.

## Test Plan

### New/Modified Tests

None in this ticket. The filter is tested indirectly by:
- S03PLATARIDE-001 unit tests (prove `matches_binding` correctness)
- S03PLATARIDE-004 integration tests (prove search rejects wrong targets in multi-entity scenarios)
- All existing golden tests (prove no regression)

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-17
- **What changed**: Added 6-line `.retain()` call in `search_candidates()` at `crates/worldwake-ai/src/search.rs:399-404`, immediately after the existing facility-use blocked filter. The filter calls `goal.key.kind.matches_binding()` to reject candidates whose authoritative targets violate goal binding.
- **Deviations from plan**: None. Implementation matched the ticket exactly.
- **Verification**: `cargo clippy --workspace` clean. `cargo test --workspace` — all 2349 tests pass, 0 failures, including all golden tests.
