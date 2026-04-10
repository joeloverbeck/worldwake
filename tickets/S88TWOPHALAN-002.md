# S88TWOPHALAN-002: Add `preferred_operator_boost` to ExecutionBudget

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — extends existing component with new field
**Deps**: None

## Problem

The two-phase planner's dual open list (S88 D6) needs a per-agent parameter controlling how many consecutive preferred-operator expansions occur before alternating to the regular queue. Without this field, the dual frontier cannot be parameterized per-agent (FND-22).

## Assumption Reassessment (2026-04-11)

1. `ExecutionBudget` exists at `crates/worldwake-core/src/execution_budget.rs:6` with 2 fields (`beam_width`, `max_prerequisite_locations`). Default impl at line 11. Component registration via `impl Component` at line 20. Struct literal construction sites: 20 occurrences across 9 files.
2. `AgentDef` uses `Option<ExecutionBudget>` at `crates/worldwake-cli/src/scenario/types.rs:88`. `spawn_agent()` applies via `unwrap_or_default()` at `crates/worldwake-cli/src/scenario/mod.rs:1449`. No CLI changes needed.
3. Shared boundary: `ExecutionBudget` is defined in `worldwake-core` and read by `worldwake-ai`. Adding a field with a default is non-breaking.

## Architecture Check

1. Same clean extension pattern as S88TWOPHALAN-001. No new components or registration needed.
2. No backwards-compatibility shims.

## Verification Layers

1. Field exists with correct type and default → focused unit test in `execution_budget.rs`
2. Serialization roundtrip → existing `execution_budget_roundtrips_through_bincode` test (updated)
3. Single-layer ticket (core data struct extension) — no cross-layer mapping needed.

## What to Change

### 1. Add field to `ExecutionBudget` struct

In `crates/worldwake-core/src/execution_budget.rs`, add after `max_prerequisite_locations`:

```rust
/// Number of consecutive preferred-operator expansions before alternating
/// to the regular queue. Higher values focus search more aggressively on
/// landmark-derived actions. 0 = no boosting (dual queue alternates 1:1).
pub preferred_operator_boost: u8,
```

### 2. Update Default impl

Add `preferred_operator_boost: 2` to the Default impl.

### 3. Update default assertion test

Add `assert_eq!(budget.preferred_operator_boost, 2);` to `execution_budget_default_matches_split_defaults`.

### 4. Update bincode roundtrip test

Add `preferred_operator_boost: 4` to the test fixture in `execution_budget_roundtrips_through_bincode`.

### 5. Update all struct literal construction sites

Every `ExecutionBudget { ... }` literal across 9 files must include the new field. Files:

- `crates/worldwake-core/src/execution_budget.rs` (tests: lines 49, 66)
- `crates/worldwake-core/src/delta.rs` (line 577)
- `crates/worldwake-cli/src/handlers/persistence.rs` (line 191)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (lines 1272–1273)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (lines 112–113)
- `crates/worldwake-ai/src/goal_model.rs` (lines 2377–2378)
- `crates/worldwake-ai/src/search/tests.rs` (lines 63–64)
- `crates/worldwake-ai/tests/conformance_execution_budget.rs` (lines 252, 280, 287)
- `crates/worldwake-ai/tests/golden_offices.rs` (lines 465, 725)

## Files to Touch

- `crates/worldwake-core/src/execution_budget.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-cli/src/handlers/persistence.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/tests/conformance_execution_budget.rs` (modify)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)

## Out of Scope

- Using `preferred_operator_boost` in the planner (that's S88TWOPHALAN-007)
- Modifying scenario files to set non-default values
- Any behavioral changes to planning

## Acceptance Criteria

### Tests That Must Pass

1. `execution_budget_default_matches_split_defaults` — asserts default is 2
2. `execution_budget_roundtrips_through_bincode` — roundtrip with non-default value
3. `execution_budget_registers_for_agents` — unchanged, still passes
4. Existing suite: `cargo test -p worldwake-core -- execution_budget`
5. Existing suite: `cargo test --workspace`

### Invariants

1. `ExecutionBudget::default().preferred_operator_boost == 2`
2. All existing tests pass without behavioral changes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/execution_budget.rs::execution_budget_default_matches_split_defaults` — add assertion for new field default
2. `crates/worldwake-core/src/execution_budget.rs::execution_budget_roundtrips_through_bincode` — add field to test fixture

### Commands

1. `cargo test -p worldwake-core -- execution_budget`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
