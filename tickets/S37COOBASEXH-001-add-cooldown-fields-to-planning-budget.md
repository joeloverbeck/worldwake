# S37COOBASEXH-001: Add cooldown fields to PlanningBudget

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — PlanningBudget struct, Default impl
**Deps**: None (first ticket in S37 chain)

## Problem

`PlanningBudget` has no cooldown parameters. The cooldown-based exhaustion system (S37) needs per-agent-tunable `initial_cooldown_ticks` and `max_cooldown_ticks` fields so that retry frequency is profile-driven (P2, P20) rather than hardcoded.

## Assumption Reassessment (2026-03-29)

1. `PlanningBudget` is defined in `crates/worldwake-ai/src/budget.rs:5-16` with 10 fields. Default impl at lines 18-33. Focused test `planning_budget_default_matches_ticket_values` at line 41 asserts all current defaults. Bincode round-trip test at line 57.
2. Spec S37 Section 1 specifies `initial_cooldown_ticks: u32` (default 4) and `max_cooldown_ticks: u32` (default 64).
3. Single-layer ticket: only touches the budget struct and its tests. No cross-system boundary.
4. N/A — no golden scenario.
5. N/A — no planner/golden-driven scope.
6. N/A — not an AI regression ticket.
7. N/A — no ordering dependency.
8. N/A — no heuristic removal.
9. N/A — not a stale-request ticket.
10. N/A — not a political office-claim ticket.
11. N/A — no ControlSource manipulation.
12. N/A — no golden scenario isolation.
13. No adjacent contradictions found.
14. No mismatch — spec aligns with current code.
15. N/A — no cumulative state arithmetic.

## Architecture Check

1. Adding two `u32` fields to an existing profile struct is the minimal change. No new types, no new files. Follows the existing pattern of all other `PlanningBudget` fields.
2. No backward-compatibility aliasing. Old saves will fail at version check (handled by S37COOBASEXH-007).

## Verification Layers

1. `initial_cooldown_ticks == 4` default → focused unit test assertion
2. `max_cooldown_ticks == 64` default → focused unit test assertion
3. Bincode round-trip preserves new fields → existing `planning_budget_roundtrips_through_bincode` test (updated)
4. Single-layer ticket: budget struct definition + defaults. No additional layer mapping applicable.

## What to Change

### 1. Add fields to `PlanningBudget` struct

In `crates/worldwake-ai/src/budget.rs`, add two fields after `structural_block_ticks`:

```rust
/// Initial cooldown in ticks after first budget exhaustion.
/// Doubles per consecutive failure up to `max_cooldown_ticks`.
pub initial_cooldown_ticks: u32,
/// Maximum cooldown in ticks (cap for exponential doubling).
pub max_cooldown_ticks: u32,
```

### 2. Update `Default` impl

Add defaults:
```rust
initial_cooldown_ticks: 4,
max_cooldown_ticks: 64,
```

### 3. Update focused tests

- Update `planning_budget_default_matches_ticket_values` to assert the two new defaults.
- The bincode round-trip test should pass without changes (new fields are part of the struct).

## Files to Touch

- `crates/worldwake-ai/src/budget.rs` (modify)

## Out of Scope

- `ExhaustionEntry` struct changes (S37COOBASEXH-002)
- Any planning logic changes (S37COOBASEXH-003, -004, -005)
- Decision trace changes (S37COOBASEXH-006)
- Save/load version bump (S37COOBASEXH-007)
- Any file outside `budget.rs`

## Acceptance Criteria

### Tests That Must Pass

1. `planning_budget_default_matches_ticket_values` — asserts `initial_cooldown_ticks == 4` and `max_cooldown_ticks == 64`
2. `planning_budget_roundtrips_through_bincode` — new fields survive bincode serialize/deserialize
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. All existing `PlanningBudget` field defaults unchanged
2. `PlanningBudget` remains `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/budget.rs::planning_budget_default_matches_ticket_values` — add assertions for new fields
2. No new test files needed

### Commands

1. `cargo test -p worldwake-ai -- planning_budget`
2. `cargo clippy --workspace && cargo test --workspace`
