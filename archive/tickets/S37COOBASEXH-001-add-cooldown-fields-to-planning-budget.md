# S37COOBASEXH-001: Add cooldown fields to PlanningBudget

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — PlanningBudget struct, Default impl
**Deps**: None (first ticket in S37 chain)

## Problem

`PlanningBudget` has no cooldown parameters. The cooldown-based exhaustion system (S37) needs per-agent-tunable `initial_cooldown_ticks` and `max_cooldown_ticks` fields so that retry frequency is profile-driven (P2, P20) rather than hardcoded.

## Assumption Reassessment (2026-03-29)

1. `PlanningBudget` is defined in `crates/worldwake-ai/src/budget.rs` with 10 fields today. `impl Default for PlanningBudget` and the focused tests `budget::tests::planning_budget_default_matches_ticket_values` and `budget::tests::planning_budget_roundtrips_through_bincode` already exist and were verified via `cargo test -p worldwake-ai -- --list`.
2. `archive/specs/S37-cooldown-based-exhaustion.md` Section "Deliverables / 1. Add cooldown parameters to PlanningBudget" still specifies `initial_cooldown_ticks: u32` with default `4` and `max_cooldown_ticks: u32` with default `64`.
3. Scope remains single-layer at the profile/config boundary. No shared cross-crate contract changes are required in this ticket because the live `PlanningBudget` struct literals checked in `crates/worldwake-ai/src/agent_tick/planning.rs`, `crates/worldwake-ai/src/search/tests.rs`, `crates/worldwake-ai/src/agent_tick/tests.rs`, `crates/worldwake-ai/tests/`, and `crates/worldwake-cli/src/handlers/persistence.rs` all use `..PlanningBudget::default()`, so the new fields propagate without per-callsite edits.
4. N/A — no golden scenario.
5. N/A — no planner/golden-driven scope.
6. N/A — not an AI regression ticket.
7. N/A — no ordering dependency.
8. N/A — no heuristic removal.
9. N/A — not a stale-request ticket.
10. N/A — not a political office-claim ticket.
11. N/A — no ControlSource manipulation.
12. N/A — no golden scenario isolation.
13. No adjacent contradictions found during reassessment.
14. Current code intentionally lags the S37 spec here: the two cooldown fields are still missing. That is the intended gap this ticket closes.
15. N/A — no cumulative state arithmetic.

## Architecture Check

1. Adding two `u32` fields to the existing profile struct is cleaner than introducing separate cooldown config objects or hardcoded constants in planning code. The planner already reads retry/search limits from `PlanningBudget`; keeping cooldown knobs on that same per-agent profile preserves one canonical configuration boundary.
2. The verified callsite shape means this remains a surgical change in `budget.rs` rather than a scattered constructor-update sweep, which is the more robust and maintainable architecture.
3. No backward-compatibility aliasing or shadow config path is introduced. Save/load compatibility remains deferred to `S37COOBASEXH-007`.

## Verification Layers

1. `initial_cooldown_ticks == 4` default -> `budget::tests::planning_budget_default_matches_ticket_values`
2. `max_cooldown_ticks == 64` default -> `budget::tests::planning_budget_default_matches_ticket_values`
3. Bincode round-trip preserves the new fields -> `budget::tests::planning_budget_roundtrips_through_bincode`
4. Constructor blast radius stays contained through `..PlanningBudget::default()` usage -> compile + `cargo test -p worldwake-ai -- budget::tests::planning_budget`

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
- Keep the bincode round-trip equality test green so serialized `PlanningBudget` values include the new fields without adding a second serialization path.

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

1. `budget::tests::planning_budget_default_matches_ticket_values` — modified to lock the new default cooldown values at the profile boundary.
2. `budget::tests::planning_budget_roundtrips_through_bincode` — modified indirectly by the struct shape change; kept as the regression guard that serialization preserves the new fields.

### Commands

1. `cargo test -p worldwake-ai -- planning_budget`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed: added `initial_cooldown_ticks` and `max_cooldown_ticks` to `crates/worldwake-ai/src/budget.rs`, set their defaults to `4` and `64`, and extended the focused default-value test to assert both fields.
- Deviations from original plan: none on code scope; reassessment confirmed that no constructor callsite edits were required because the live `PlanningBudget` literals already rely on `..PlanningBudget::default()`.
- Verification results: `cargo test -p worldwake-ai -- planning_budget`, `cargo test -p worldwake-ai`, `cargo clippy --workspace`, and `cargo test --workspace` all passed.
