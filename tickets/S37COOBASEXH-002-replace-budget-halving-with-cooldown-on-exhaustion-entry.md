# S37COOBASEXH-002: Replace budget-halving with cooldown on ExhaustionEntry

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — ExhaustionEntry struct, methods, factory constructors
**Deps**: S37COOBASEXH-001 (needs `PlanningBudget` cooldown fields)

## Problem

`ExhaustionEntry` uses `consecutive_budget_exhaustions` to halve search budget via `effective_max_expansions()`. This makes search *shallower* on repeated failure — the wrong shape. Need to replace with cooldown-based retry fields (`next_retry_tick`, `consecutive_failures`) and methods (`is_retry_eligible()`, `record_budget_exhaustion()`).

## Assumption Reassessment (2026-03-29)

1. `ExhaustionEntry` is defined in `crates/worldwake-ai/src/decision_runtime.rs:64-74`. Methods at lines 76-120. Field `consecutive_budget_exhaustions: u8` at line 73 with `#[serde(default)]`. `effective_max_expansions()` at lines 106-109. `suppresses_planning()` at lines 112-114. `is_budget_retry_pending()` at lines 117-119. Factory methods `frontier_exhausted()` at 78-88 and `budget_retry_pending()` at 91-101.
2. Spec S37 Section 2 specifies the replacement struct, new methods, and updated factory signatures. `budget_retry_pending()` gains `current_tick: Tick` and `budget: &PlanningBudget` parameters.
3. `ExhaustionEntry` is used in `planning.rs` (caller sites) and `exhaustion.rs` (invalidation). This ticket changes the data model; call-site adaptation is in S37COOBASEXH-003/004/005.
4. N/A — no golden scenario.
5. N/A — not a planner-driven ticket.
6. N/A — not an AI regression.
7. N/A — no ordering dependency.
8. Removing `effective_max_expansions()` and `is_budget_retry_pending()`: these are the budget-halving heuristics being replaced by cooldown. The cooldown mechanism (this ticket) is the substrate. No unrelated regressions because downstream callers are updated in subsequent tickets.
9. N/A — not a stale-request ticket.
10. N/A — not a political ticket.
11. N/A — no ControlSource manipulation.
12. N/A — no golden scenario.
13. Downstream callers of removed methods (`effective_max_expansions`, `is_budget_retry_pending`) will fail to compile until S37COOBASEXH-003/004/005 land. This is expected — tickets should be implemented in order.
14. No mismatch with spec.
15. Cooldown arithmetic: `initial_cooldown << (consecutive_failures - 1)`, capped at `max_cooldown_ticks`. With defaults (4, 64): 4→8→16→32→64→64. Shift capped at 6 to prevent `u32` overflow.

## Architecture Check

1. Replacing a halving counter with cooldown tick + failure counter is a clean substitution. Same struct, different semantics. `record_budget_exhaustion()` encapsulates the cooldown formula.
2. No backward-compatibility shims. Old `consecutive_budget_exhaustions` field removed. Save format handled by S37COOBASEXH-007.

## Verification Layers

1. First budget exhaustion → `next_retry_tick == current_tick + initial_cooldown_ticks` → focused unit test
2. Consecutive exhaustion → cooldown doubles → focused unit test
3. Cooldown caps at `max_cooldown_ticks` → focused unit test
4. `is_retry_eligible(tick)` returns correct bool → focused unit test
5. `FrontierExhausted` always ineligible → focused unit test
6. Single-layer ticket for data model. Caller integration tested in downstream tickets.

## What to Change

### 1. Replace `consecutive_budget_exhaustions` field

In `crates/worldwake-ai/src/decision_runtime.rs`, on `ExhaustionEntry`:

- Remove field `consecutive_budget_exhaustions: u8`
- Add field `next_retry_tick: Option<Tick>` (with `#[serde(default)]`)
- Add field `consecutive_failures: u8` (with `#[serde(default)]`)

### 2. Remove `effective_max_expansions()` and `is_budget_retry_pending()`

Delete these two methods entirely.

### 3. Update `suppresses_planning()`

Keep as-is (unchanged behavior for `FrontierExhausted`).

### 4. Add `is_retry_eligible(&self, current_tick: Tick) -> bool`

Returns `false` for `FrontierExhausted`. For `BudgetRetryPending`, returns `true` when `current_tick >= next_retry_tick` (or `next_retry_tick` is `None`).

### 5. Add `record_budget_exhaustion(&mut self, current_tick: Tick, budget: &PlanningBudget)`

Increments `consecutive_failures`, computes cooldown via `initial_cooldown_ticks << (failures - 1)` capped at `max_cooldown_ticks`, sets `next_retry_tick`.

### 6. Update factory methods

- `frontier_exhausted()`: set `next_retry_tick: None`, `consecutive_failures: 0`
- `budget_retry_pending()`: add `current_tick: Tick` and `budget: &PlanningBudget` params. Call `record_budget_exhaustion()` internally.

### 7. Add focused tests

Tests for cooldown progression, cap, eligibility, and factory methods.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify)

## Out of Scope

- `planning.rs` caller adaptations (S37COOBASEXH-003, -004, -005)
- `exhaustion.rs` invalidation logic (no changes needed per spec Section 7)
- Decision trace changes (S37COOBASEXH-006)
- Save/load version bump (S37COOBASEXH-007)
- `PlanningBudget` changes (S37COOBASEXH-001)
- Any golden test changes

## Acceptance Criteria

### Tests That Must Pass

1. First budget exhaustion sets cooldown to `initial_cooldown_ticks` (default 4 ticks)
2. Second consecutive exhaustion doubles cooldown to 8 ticks
3. Third consecutive exhaustion sets cooldown to 16 ticks
4. Cooldown caps at `max_cooldown_ticks` (default 64) after sufficient consecutive failures
5. `is_retry_eligible()` returns false when `current_tick < next_retry_tick`
6. `is_retry_eligible()` returns true when `current_tick >= next_retry_tick`
7. `is_retry_eligible()` returns false for `FrontierExhausted` regardless of tick
8. Custom `PlanningBudget` cooldown values respected (e.g., initial=10, max=100)
9. `FrontierExhausted` factory sets `consecutive_failures: 0`, `next_retry_tick: None`

### Invariants

1. `ExhaustionEntry` remains `Clone + Debug + Eq + PartialEq + Ord + PartialOrd + Serialize + Deserialize`
2. `FrontierExhausted` entries are never retry-eligible (only invalidation clears them)
3. Cooldown is purely deterministic tick arithmetic — no wall-clock, no randomness

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` — new focused tests for cooldown progression, cap, eligibility, factory methods
2. Existing tests referencing `consecutive_budget_exhaustions` or `effective_max_expansions` must be updated or removed

### Commands

1. `cargo test -p worldwake-ai -- exhaustion`
2. `cargo clippy --workspace && cargo test -p worldwake-ai`
