# PERAGRSTY-002: Replace `PlanningBudget` with per-agent `ReasoningProfile` in AI pipeline

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI pipeline parameter resolution, driver serialization, CLI call sites
**Deps**: PERAGRSTY-001

## Problem

`AgentTickDriver` stores a single shared `PlanningBudget` and passes it to all agents identically. With `ReasoningProfile` now available as a per-agent component (PERAGRSTY-001), the AI pipeline must be updated to resolve the profile from the world's component tables for each agent. `PlanningBudget` must be fully removed (Principle 28 — no backward compatibility layers).

## Assumption Reassessment (2026-04-03)

1. `AgentTickDriver` stores `budget: PlanningBudget` at `crates/worldwake-ai/src/agent_tick/mod.rs:57`. Constructor at line 71 takes `budget: PlanningBudget`. Serialized state `AgentTickDriverState` at lines 63-67 also contains `budget: PlanningBudget`.
2. `from_saved_runtime()` at line 80 constructs `Self::new(PlanningBudget::default())` — ignores the deserialized budget.
3. Functions taking `&PlanningBudget`:
   - `search_plan()` at `crates/worldwake-ai/src/search/mod.rs:83`
   - `handle_plan_failure()` at `crates/worldwake-ai/src/failure_handling.rs:34`
   - `blocking_fact_ttl()` at `crates/worldwake-ai/src/failure_handling.rs:790`
   - `record_budget_exhaustion()` at `crates/worldwake-ai/src/decision_runtime.rs:124`
   - `build_candidate_plans()` at `crates/worldwake-ai/src/agent_tick/planning.rs` (receives budget)
   - `goal_switch_margin_details()` at `crates/worldwake-ai/src/agent_tick/active_action.rs:150`
4. CLI call sites constructing `AgentTickDriver::new(PlanningBudget::default())`:
   - `crates/worldwake-cli/src/main.rs:59,116`
   - `crates/worldwake-cli/src/handlers/tick.rs:236`
   - `crates/worldwake-cli/src/handlers/actions.rs:364`
   - `crates/worldwake-cli/tests/integration.rs:48,427,434`
5. `goal_switch_margin_details()` has a two-tier precedence: `IntentionDispositionProfile.commitment_switch_margin` overrides when active frame exists; otherwise falls back to `budget.switch_margin_permille`. After migration, the fallback reads `ReasoningProfile.switch_margin` instead. Semantics unchanged.
6. `SAVE_FORMAT_VERSION` is currently 13 at `crates/worldwake-sim/src/save_load.rs:6`.
7. 26 files reference `PlanningBudget` across the workspace. ~11 golden test files use `PlanningBudget::default()`.
8. Not a planner/golden/ranking/stale-request/political/ControlSource/heuristic-removal ticket — domain-specific precision items 5-15 are N/A. This is a mechanical signature migration.

## Architecture Check

1. Full replacement is cleaner than a bridge or shim. Every `&PlanningBudget` parameter becomes `&ReasoningProfile`. The driver resolves the profile once per agent per tick from the world's component tables, falling back to `ReasoningProfile::default()` — same pattern as `PerceptionProfile` resolution.
2. No backward-compatibility aliasing. `PlanningBudget` and `budget.rs` are deleted entirely. `AgentTickDriverState` loses its `budget` field. No `From` impl.

## Verification Layers

1. All existing tests pass with identical behavior (agents without explicit profiles get `Default`) -> `cargo test --workspace`
2. `AgentTickDriver` no longer stores budget -> structural: field removed from struct and serialized state
3. No remaining `PlanningBudget` references -> `grep -r PlanningBudget crates/` returns zero hits
4. Save/load round-trip with new format -> focused test serializing/deserializing `AgentTickDriverState` without budget field
5. `switch_margin` fallback precedence unchanged -> existing `goal_switch_margin_details` tests still pass
6. Mixed-layer boundary: the shared contract is `ReasoningProfile` resolved from `World` component tables. AI pipeline reads it; core stores it. No cross-system mutation.

## What to Change

### 1. Remove `PlanningBudget` from `AgentTickDriver`

In `crates/worldwake-ai/src/agent_tick/mod.rs`:
- Remove `budget: PlanningBudget` field from `AgentTickDriver` struct (line 57).
- Update `AgentTickDriver::new()` to take no budget parameter.
- Remove `budget: PlanningBudget` field from `AgentTickDriverState` (line 66).
- Update `from_saved_runtime()` to not expect budget in deserialized state.

### 2. Add per-agent profile resolution

In `crates/worldwake-ai/src/agent_tick/mod.rs`, at the start of the per-agent decision path:
```rust
let reasoning = world.get_reasoning_profile(agent)
    .cloned()
    .unwrap_or_default();
```
Pass `&reasoning` to all downstream consumers where `&self.budget` was previously used.

### 3. Update function signatures

Replace `&PlanningBudget` with `&ReasoningProfile` in:
- `search_plan()` in `crates/worldwake-ai/src/search/mod.rs`
- `handle_plan_failure()` in `crates/worldwake-ai/src/failure_handling.rs`
- `blocking_fact_ttl()` in `crates/worldwake-ai/src/failure_handling.rs`
- `record_budget_exhaustion()` in `crates/worldwake-ai/src/decision_runtime.rs`
- `build_candidate_plans()` in `crates/worldwake-ai/src/agent_tick/planning.rs`
- `goal_switch_margin_details()` and `effective_goal_switch_margin()` in `crates/worldwake-ai/src/agent_tick/active_action.rs`
- Any helper functions in `crates/worldwake-ai/src/agent_tick/frame.rs` that access budget fields

Update all field accesses from `budget.switch_margin_permille` to `profile.switch_margin` (or equivalent renamed field).

### 4. Update CLI call sites

In all files that construct `AgentTickDriver::new(PlanningBudget::default())`:
- `crates/worldwake-cli/src/main.rs` — change to `AgentTickDriver::new()`
- `crates/worldwake-cli/src/handlers/tick.rs` — same
- `crates/worldwake-cli/src/handlers/actions.rs` — same
- `crates/worldwake-cli/tests/integration.rs` — same

Remove `use worldwake_ai::PlanningBudget;` from these files.

### 5. Delete `PlanningBudget`

Delete `crates/worldwake-ai/src/budget.rs` entirely. Remove `pub mod budget;` and `pub use budget::PlanningBudget;` from `crates/worldwake-ai/src/lib.rs`.

### 6. Update test files

All test files in `worldwake-ai` that construct or reference `PlanningBudget::default()` must switch to `ReasoningProfile::default()`. This includes:
- `crates/worldwake-ai/src/agent_tick/tests.rs`
- `crates/worldwake-ai/src/search/tests.rs`
- `crates/worldwake-ai/src/failure_handling.rs` (test module)
- Golden test files in `crates/worldwake-ai/tests/`

### 7. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:6`, bump from 13 to 14. Add a save/load round-trip test that:
- Creates a world with an agent that has a non-default `ReasoningProfile`
- Saves and loads
- Verifies the profile is preserved

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — remove budget, add profile resolution)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — signature change)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — signature change, field rename)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — field accesses)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — PlanningBudget -> ReasoningProfile)
- `crates/worldwake-ai/src/search/mod.rs` (modify — signature change)
- `crates/worldwake-ai/src/search/tests.rs` (modify — PlanningBudget -> ReasoningProfile)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — signature change + tests)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — signature change)
- `crates/worldwake-ai/src/budget.rs` (delete)
- `crates/worldwake-ai/src/lib.rs` (modify — remove budget module/export)
- `crates/worldwake-cli/src/main.rs` (modify — constructor call)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify — constructor call)
- `crates/worldwake-cli/src/handlers/actions.rs` (modify — constructor call)
- `crates/worldwake-cli/tests/integration.rs` (modify — constructor calls)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump version)
- Golden test files in `crates/worldwake-ai/tests/` (modify — PlanningBudget -> ReasoningProfile)

## Out of Scope

- Adding non-default `ReasoningProfile` values to any agent (agents all get `Default` — behavioral parity with current state)
- Golden test proving diversity with different profiles (PERAGRSTY-003)
- Any changes to `IntentionDispositionProfile` or the `commitment_switch_margin` precedence logic (unchanged)

## Acceptance Criteria

### Tests That Must Pass

1. All existing `worldwake-ai` tests pass with zero behavioral change
2. All existing golden tests pass with zero behavioral change
3. Save/load round-trip preserves non-default `ReasoningProfile` on an agent
4. `grep -r 'PlanningBudget' crates/` returns zero results
5. Existing suite: `cargo test --workspace`

### Invariants

1. Agents without an explicit `ReasoningProfile` behave identically to pre-migration behavior
2. `AgentTickDriver` does not store any per-agent reasoning parameters — resolution is always from the world's component tables
3. `switch_margin` fallback precedence: `IntentionDispositionProfile.commitment_switch_margin` (when active frame) > `ReasoningProfile.switch_margin` (otherwise) > `ReasoningProfile::default().switch_margin` (when no component)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — update all `PlanningBudget::default()` references to `ReasoningProfile::default()`
2. `crates/worldwake-ai/src/search/tests.rs` — same
3. `crates/worldwake-ai/src/failure_handling.rs` (tests) — same
4. Save/load round-trip test in appropriate location — verify `ReasoningProfile` survives save/load

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-cli`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
