# S21-001: Define and register ActiveGoal, JourneyCommitment, FacilityQueueIntents components in worldwake-core

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new component types, component_tables, component_schema, save format version bump
**Deps**: S20 (completed)

## Problem

`AgentDecisionRuntime` holds causally relevant state (active goal, journey commitment, facility intents) that is lost on save/load because `AgentTickDriver` is not part of `SimulationState`. This ticket defines the three new ECS components and the supporting types they need, but does NOT migrate any AI code.

## Assumption Reassessment (2026-03-23)

1. `BlockedIntentMemory` registration in `component_schema.rs` (lines 333–356) is the canonical pattern for Agent-only components. Confirmed present.
2. `component_tables.rs` uses `define_component_tables_struct!` and `define_component_table_impls!` macros (lines 72–96) to generate `BTreeMap<EntityId, T>` storage. Confirmed.
3. `GoalKey` lives in `worldwake-core::goal` (line 66) and derives `Serialize, Deserialize`. No transitive changes needed for `ActiveGoal`.
4. `ActionDefId` lives in `worldwake-core::ids` (line 86) and derives `Serialize, Deserialize` via the macro. No transitive changes for `QueuedFacilityIntent`.
5. `JourneyCommitmentState` lives in `worldwake-ai::decision_runtime` (lines 7–12), derives `Copy, Clone, Debug, Default, Eq, PartialEq` — needs `Serialize, Deserialize` added when moved to core.
6. `QueuedFacilityIntent` lives in `worldwake-ai::decision_runtime` (lines 50–54), derives `Clone, Copy, Debug, Eq, PartialEq` — needs `Serialize, Deserialize` added when moved to core.
7. `SAVE_FORMAT_VERSION` is currently `4` in `crates/worldwake-sim/src/save_load.rs` (line 6).
8. Version-check tests use `SAVE_FORMAT_VERSION + 1` (relative), so the bump to 5 won't break them.

## Architecture Check

1. Following the established `BlockedIntentMemory` pattern ensures consistency — no new macro or registration mechanism needed.
2. No backward-compatibility shims. Types move from AI to core; temporary re-exports may be added in the AI crate's `decision_runtime.rs` for compilation but are explicitly scoped for removal in S21-006.

## Verification Layers

1. New components serialize/deserialize correctly → `save_to_bytes_roundtrip_preserves_full_nondefault_state` (existing test, exercises all `ComponentTables` fields)
2. Schema restricts to Agent-only → `component_schema` kind-check closure `|kind| kind == EntityKind::Agent`
3. `SAVE_FORMAT_VERSION == 5` → `load_rejects_wrong_version` test (existing, uses relative offset)
4. Workspace compiles → `cargo build --workspace`

## What to Change

### 1. Move types from `worldwake-ai::decision_runtime` to `worldwake-core`

Move `JourneyCommitmentState` and `QueuedFacilityIntent` to a new module `crates/worldwake-core/src/intention.rs` (or add to existing `goal.rs`). Add `Serialize, Deserialize` derives. Re-export from `worldwake-core::lib.rs`.

### 2. Define new component structs in `worldwake-core`

In `crates/worldwake-core/src/intention.rs` (new file):
- `ActiveGoal { goal_key: GoalKey, adopted_at: Tick }` — derives `Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize`, impl `Component`.
- `JourneyCommitment { committed_goal: GoalKey, destination: EntityId, state: JourneyCommitmentState, established_at: Tick, last_progress_tick: Option<Tick>, consecutive_blocked_leg_ticks: u32 }` — derives same, impl `Component`.
- `FacilityQueueIntents { intents: BTreeMap<EntityId, QueuedFacilityIntent> }` — derives `Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize`, impl `Component`.

### 3. Register in component_tables.rs

Add three entries to `define_component_tables_struct!` and `define_component_table_impls!` following the `BlockedIntentMemory` pattern.

### 4. Register in component_schema.rs

Add three entries restricted to `EntityKind::Agent`, following the `BlockedIntentMemory` block.

### 5. Add temporary re-exports in worldwake-ai

In `crates/worldwake-ai/src/decision_runtime.rs`, replace the original type definitions with `pub use worldwake_core::{JourneyCommitmentState, QueuedFacilityIntent};` so downstream AI code compiles without changes in this ticket.

### 6. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs`, change `SAVE_FORMAT_VERSION` from `4` to `5`.

## Files to Touch

- `crates/worldwake-core/src/intention.rs` (new — component structs and relocated types)
- `crates/worldwake-core/src/lib.rs` (modify — declare `intention` module, re-export public types)
- `crates/worldwake-core/src/component_tables.rs` (modify — add 3 entries to macros)
- `crates/worldwake-core/src/component_schema.rs` (modify — add 3 Agent-only registrations)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — remove original type defs, add re-exports from core)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump version 4→5)

## Out of Scope

- Migrating reads/writes in AI crate code (S21-002, S21-003, S21-004)
- Removing promoted fields from `AgentDecisionRuntime` (S21-002/003/004)
- Refactoring journey helper methods (S21-002)
- Writing new golden save/load tests (S21-005)
- Any changes to `agent_tick/`, `plan_selection.rs`, `failure_handling.rs`, `interrupts.rs`, `goal_switching.rs`
- Any changes to `GoldenHarness` or `from_simulation_state`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build --workspace` — compiles with new components registered and re-exports in place
2. `cargo test -p worldwake-core` — all existing core tests pass; new components participate in schema tests
3. `cargo test -p worldwake-sim` — `load_rejects_wrong_version` passes with v5; `save_to_bytes_roundtrip_preserves_full_nondefault_state` passes
4. `cargo test -p worldwake-ai` — all existing AI tests pass (re-exports keep imports working)
5. `cargo clippy --workspace` — no new warnings

### Invariants

1. All three new components are restricted to `EntityKind::Agent` only
2. `SAVE_FORMAT_VERSION == 5`
3. No behavioral change — no existing test should change its outcome
4. `JourneyCommitmentState` and `QueuedFacilityIntent` derive `Serialize, Deserialize` in their new core location
5. The `ActiveGoal`, `JourneyCommitment`, and `FacilityQueueIntents` types are public exports of `worldwake-core`

## Test Plan

### New/Modified Tests

1. None required — this is a structural/registration ticket. Existing macro-based schema tests and save/load tests automatically exercise the new entries.

### Commands

1. `cargo build --workspace`
2. `cargo test -p worldwake-core`
3. `cargo test -p worldwake-sim`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-23
- **What changed**:
  - Created `crates/worldwake-core/src/intention.rs` with `JourneyCommitmentState`, `QueuedFacilityIntent` (relocated from AI crate, gained `Serialize, Deserialize`), `ActiveGoal`, `JourneyCommitment`, `FacilityQueueIntents` (new component structs)
  - Registered all three components in `component_schema.rs` and `component_tables.rs` (Agent-only, `txn_simple_set`)
  - Added imports in `delta.rs`, `world.rs`, `component_tables.rs` for macro expansion
  - Added `ComponentValue` samples and `ComponentKind::ALL` entries in `delta.rs`
  - Replaced original type definitions in `decision_runtime.rs` with `pub use worldwake_core::{JourneyCommitmentState, QueuedFacilityIntent}`
  - Bumped `SAVE_FORMAT_VERSION` from 4 to 5
- **Deviations**: None — implemented exactly as specified
- **Verification**: `cargo build --workspace` clean, `cargo test -p worldwake-core` (8+3 pass), `cargo test -p worldwake-sim` (352 pass), `cargo test -p worldwake-ai` (32 pass), `cargo clippy --workspace` (no warnings)
