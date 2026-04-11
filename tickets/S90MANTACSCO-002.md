# S90MANTACSCO-002: Mandatory tactical goal fail-fast

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — planner search internals (`worldwake-ai`)
**Deps**: S90MANTACSCO-001

## Problem

If a future code change introduces a new `TacticalSubGoal` variant that `from_strategic_step` doesn't handle, the search would run unscoped with explosive candidate counts. A fail-fast guard prevents this by returning `FrontierExhausted` immediately when the strategic plan has steps but no tactical goal was produced.

## Assumption Reassessment (2026-04-11)

1. `TacticalGoal::from_strategic_step` call confirmed at `mod.rs:267-270`. After 001, the signature includes `snapshot`.
2. `PlanSearchResult::FrontierExhausted { expansions_used: u16 }` confirmed at `mod.rs:172`. Already has the `expansions_used` field — no variant changes needed.
3. Shared boundary: the `search_plan` function in `mod.rs`. The fail-fast guard goes between the `from_strategic_step` call (line 270) and the frontier initialization (line 272+).

## Architecture Check

1. A fail-fast guard is structurally simpler and more robust than trying to make every future `TacticalSubGoal` variant produce a tactical goal. Defense in depth: 001 fixes the current bypass, 002 prevents future bypasses.
2. No backwards-compatibility shims. The guard is a new check, not a wrapper around old behavior.

## Verification Layers

1. Fail-fast when strategic steps exist but no tactical goal → focused unit test (in 004)
2. Local goals unaffected (empty steps or None plan) → existing S88/S89 tests pass unchanged
3. Single-layer ticket: fail-fast is planner-internal, no cross-system interaction

## What to Change

### 1. Add fail-fast guard after `from_strategic_step` call

**File**: `crates/worldwake-ai/src/search/mod.rs`

After the `TacticalGoal::from_strategic_step()` call (line 267-270, post-001), add:

```rust
if strategic_plan
    .as_ref()
    .is_some_and(|plan| !plan.steps.is_empty())
    && tactical_goal.is_none()
{
    return PlanSearchResult::FrontierExhausted {
        expansions_used: 0,
    };
}
```

Local goals (strategic plan with empty steps or `None`) are exempt — they run unscoped as before because their candidate counts are bounded by local affordances.

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify)

## Out of Scope

- Modifying `PlanSearchResult` variants (both already have `expansions_used`)
- Adding new `TacticalSubGoal` variants
- Changing local goal behavior

## Acceptance Criteria

### Tests That Must Pass

1. `search_fail_fast_when_strategic_steps_but_no_tactical_goal` (new, in 004)
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Any search with non-empty strategic steps and `tactical_goal == None` returns `FrontierExhausted { expansions_used: 0 }` immediately
2. Local goals (empty steps or `None` strategic plan) continue to run unscoped

## Test Plan

### New/Modified Tests

1. None in this ticket — tests are in S90MANTACSCO-004

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
