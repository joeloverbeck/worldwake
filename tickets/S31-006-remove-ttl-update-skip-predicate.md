# S31-006: Remove EXHAUSTION_SKIP_TTL and Update Skip Predicate

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI planning pipeline
**Deps**: S31-004, S31-005

## Problem

With precise per-goal invalidation in place (S31-004) and conditions captured at recording time (S31-005), the TTL-based periodic re-search is no longer needed. Goals stay cached until their specific conditions fire. This ticket removes the TTL constant, the `exhaustion_skip_active` function, and simplifies the skip predicate in `build_candidate_plans`.

## Assumption Reassessment (2026-03-27)

1. `EXHAUSTION_SKIP_TTL = 20` at `planning.rs:21`.
2. `exhaustion_skip_active` at `planning.rs:144-148` checks `current_tick - exhausted_at < TTL`.
3. The skip predicate in `build_candidate_plans` at `planning.rs:176-178` filters candidates where `exhaustion_skip_active(entry, current_tick)` returns true.
4. The replacement predicate is simpler: skip any goal where `entry.exhausted_at.is_some()` (the goal has been exhausted and its conditions have not yet fired — S31-004 removes entries when conditions fire).
5. `build_candidate_plans` no longer needs `current_tick` for the skip check (though it's still needed for snapshot construction).
6. Tests at `planning.rs:897-912` test `exhaustion_skip_active` directly — these must be removed.
7. The exponential backoff in `build_candidate_plans:214-223` (budget halving by `entry.count`) is preserved unchanged.

## Architecture Check

1. Pure removal of dead code + simplification. The TTL mechanism is superseded by condition-based invalidation.
2. No backward-compatibility concerns — the TTL was an internal implementation detail.

## Verification Layers

1. `EXHAUSTION_SKIP_TTL` no longer exists -> grep verification
2. `exhaustion_skip_active` no longer exists -> grep verification
3. Skip predicate uses `exhausted_at.is_some()` -> code review
4. Exponential backoff preserved -> existing tests for budget reduction
5. All golden tests pass -> `cargo test -p worldwake-ai`

## What to Change

### 1. Remove `EXHAUSTION_SKIP_TTL` constant from `planning.rs`

Delete line 21: `const EXHAUSTION_SKIP_TTL: u64 = 20;`

### 2. Remove `exhaustion_skip_active` function from `planning.rs`

Delete lines 144-148.

### 3. Update skip predicate in `build_candidate_plans`

Replace lines 176-178:
```rust
.is_some_and(|entry| exhaustion_skip_active(entry, current_tick))
```
with:
```rust
.is_some_and(|entry| entry.exhausted_at.is_some())
```

### 4. Remove tests for `exhaustion_skip_active`

Delete the test functions that test the TTL behavior directly (lines ~897-912).

### 5. Update test imports

Remove `exhaustion_skip_active` from test `use` statements (line ~785).

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — remove constant, function, update predicate)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — remove TTL tests, update imports)

## Out of Scope

- Golden tests (S31-007)
- Further optimization of the skip predicate
- Changes to `derive_invalidation_conditions` or `condition_changed`
- Changes to `invalidate_exhausted_goals`

## Acceptance Criteria

### Tests That Must Pass

1. `grep -r "EXHAUSTION_SKIP_TTL" crates/` returns zero results
2. `grep -r "exhaustion_skip_active" crates/` returns zero results
3. The skip predicate in `build_candidate_plans` uses `exhausted_at.is_some()`
4. Exponential backoff (budget halving by `entry.count`) still works — existing test or new unit test
5. Existing suite: `cargo test --workspace`

### Invariants

1. No TTL-based re-search exists anywhere in the codebase
2. Invalidation is purely condition-based (S31-004) — time-based clearing is removed
3. Goals with `exhausted_at.is_some()` are skipped; goals without are searched
4. Exponential backoff is preserved for re-exhausted goals

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` tests — remove `exhaustion_skip_active` tests, verify skip predicate behavior with a test that creates entries with/without `exhausted_at`

### Commands

1. `cargo test -p worldwake-ai planning`
2. `cargo clippy --workspace && cargo test --workspace`
