# S31-004: Replace `reset_exhausted_goals_if_needed` with `invalidate_exhausted_goals`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI planning pipeline
**Deps**: S31-001, S31-002, S31-003

## Problem

The current `reset_exhausted_goals_if_needed` uses a coarse dirty-bit mask to clear ALL exhausted goals on any relevant change. This ticket replaces it with `invalidate_exhausted_goals`, which iterates each cache entry and checks only its specific conditions, removing entries whose conditions have fired.

## Assumption Reassessment (2026-03-27)

1. `reset_exhausted_goals_if_needed` is at `planning.rs:311-348`. It is called once from `planning.rs:464`.
2. The function takes `&mut AgentDecisionRuntime` and `currently_in_transit: bool`. The replacement needs `&dyn GoalBeliefView` and `agent: EntityId` additionally.
3. The call site at `planning.rs:464` already has access to `view` (constructed at line 449) and `agent`.
4. When conditions fire and an entry is removed via `.retain()`, the `count` resets to 0 on next exhaustion — intentional per spec ("the world genuinely changed, so the goal deserves a full-budget re-search").
5. The old function increments `entry.count` when clearing `exhausted_at`. The new function removes the entry entirely when conditions fire, which is a stronger reset (both `count` and `exhausted_at` gone). This matches spec Section 5 semantics.
6. Under current repo policy, empty `invalidation_conditions` entries are not a lawful long-term runtime state. The existing "always invalidate empty conditions" fallback is a stale backward-compatibility assumption that should be removed once S31-005 tightens the persisted `ExhaustionEntry` contract.
7. Reassessment after S31-002: `invalidate_exhausted_goals` cannot rely on `condition_changed` with facility/blocker "always true" semantics. This ticket must pass runtime-derived booleans for facility access changes and blocker cleanup so facility-tagged and blocker-tagged goals do not self-invalidate every tick.

## Architecture Check

1. Direct replacement of one function with another. Same call site, slightly expanded parameters. Clean swap.
2. No backward-compatibility shims — the old function is deleted, not wrapped.
3. The clean boundary is: observation/runtime computes dirty domains once, then invalidation consumes those domains as facts. `invalidate_exhausted_goals` should not recompute facility signatures or blocker expiry internally.

## Verification Layers

1. Per-goal invalidation (only entries with fired conditions removed) -> unit test with mixed cache
2. Transit filtering preserved (`PositionChanged` not fired mid-transit) -> unit test
3. Facility-tagged goals are retained when no facility dirty signal is present -> unit test
4. Blocker-tagged goals are retained when no blocker-cleanup signal is present -> unit test
5. Integration: call site compiles with new parameters -> compilation
6. Existing golden tests pass -> `cargo test -p worldwake-ai`

## What to Change

### 1. Add `invalidate_exhausted_goals` to `crates/worldwake-ai/src/exhaustion.rs`

```rust
pub(crate) fn invalidate_exhausted_goals(
    exhaustion_cache: &mut BTreeMap<GoalKey, ExhaustionEntry>,
    view: &dyn GoalBeliefView,
    agent: EntityId,
    currently_in_transit: bool,
    facilities_changed: bool,
    blocker_expired: bool,
)
```

Uses `.retain()` to remove entries where any condition has changed (via `condition_changed`). After S31-005 removes the stale serde-default fallback, every lawful entry should carry explicit conditions and baseline data.

### 2. Replace call site in `planning.rs`

At `planning.rs:464`, replace:
```rust
reset_exhausted_goals_if_needed(runtime, view.in_transit_state(agent).is_some());
```
with:
```rust
invalidate_exhausted_goals(
    &mut runtime.exhaustion_cache,
    &view,
    agent,
    view.in_transit_state(agent).is_some(),
    runtime.dirty.contains(DirtySet::FACILITIES),
    runtime.dirty.contains(DirtySet::BLOCKER_CLEANUP),
);
```

### 3. Remove `reset_exhausted_goals_if_needed` from `planning.rs`

Delete the function definition (lines 311-348) and its `#[cfg(test)]` imports/usage.

### 4. Update tests in `planning.rs` that tested `reset_exhausted_goals_if_needed`

The test `reset_exhausted_goals_if_needed_clears_ttl_marker_and_preserves_backoff_history` (line 833) tests the old function. Replace with equivalent tests for `invalidate_exhausted_goals`.

## Files to Touch

- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — replace call site, delete old function)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — update tests for old function if any)

## Out of Scope

- Changes to `record_exhausted_goals` (S31-005)
- Removing `EXHAUSTION_SKIP_TTL` or `exhaustion_skip_active` (S31-006)
- Golden tests (S31-007)
- Dirty-bit refinement for `FacilitiesChanged`/`BlockerExpired`

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: cache with 3 entries, only 1 has fired condition -> only that entry removed
2. Unit test: entry with unfired conditions is retained
3. Unit test: `PositionChanged` condition not fired when `currently_in_transit == true`
4. Unit test: `FacilitiesChanged` does not remove an entry when the runtime did not report facility dirtiness
5. Unit test: `BlockerExpired` does not remove an entry when the runtime did not report blocker cleanup
6. Existing suite: `cargo test --workspace`

### Invariants

1. `reset_exhausted_goals_if_needed` no longer exists in the codebase
2. Invalidation is per-goal, not per-dirty-bit-mask
3. Facility and blocker invalidation remain driven by observed runtime state, not unconditional fallbacks
4. No regression in existing golden tests

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` (inline `#[cfg(test)]`) — selective invalidation with mixed cache
2. `crates/worldwake-ai/src/agent_tick/planning.rs` tests — remove/replace `reset_exhausted_goals_if_needed` test

### Commands

1. `cargo test -p worldwake-ai exhaustion`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo clippy --workspace && cargo test --workspace`
