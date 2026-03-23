# S20STRCLE-005: Extract agent_tick/execution.rs

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S20STRCLE-001

## Problem

The execution/input-enqueueing logic in `agent_tick/mod.rs` (~300 lines) handles converting a validated plan step into a queued input and managing start failures. Extracting it into a dedicated sub-module clarifies the "commit the decision" phase of the pipeline.

## Assumption Reassessment (2026-03-22)

1. Functions to extract (verified via grep):
   - `enqueue_valid_step_or_handle_failure()` (line 591)
   - `finalize_agent_tick()` (line 1512)
   - `resolve_step_targets()` (line 1845)
   - `committed_action_for_step()` (line 1854)
   - `apply_step_materialization_bindings()` (line 1865)
   - `persist_blocked_memory()` (line 1917)
   - `current_step()` (line 1947)
   - `plan_finished()` (line 1954)
2. All private or `pub(crate)`.
3–12. N/A — pure structural refactor.

## Architecture Check

1. These functions form the "execute the decided step" phase: enqueueing inputs, resolving step targets, applying bindings, and persisting blocked intent memory. Clear cohesion around "act on the plan."
2. No backward-compatibility shims.

## Verification Layers

1. All tests pass → `cargo test -p worldwake-ai`.
2. Single-layer ticket: code motion only.

## What to Change

### 1. Create `agent_tick/execution.rs`

Move the listed functions. Add necessary `use` imports.

### 2. Update `agent_tick/mod.rs`

- Add `mod execution;`
- Add `use execution::*;` or explicit items.
- Remove moved function bodies.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/execution.rs` (new)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)

## Out of Scope

- Extracting other pipeline stages
- Changing function signatures, visibility, or logic
- Modifying `search.rs`
- Any changes outside `worldwake-ai`
- Reorganizing tests (S20STRCLE-013)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all tests pass unchanged
2. `cargo clippy -p worldwake-ai` — no new warnings

### Invariants

1. Zero behavioral change
2. `lib.rs` re-exports continue to resolve
3. Function signatures and visibility unchanged

## Test Plan

### New/Modified Tests

1. None — code motion only; verification is command-based.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`

## Outcome

- **Completion date**: 2026-03-23
- **What changed**: Created `crates/worldwake-ai/src/agent_tick/execution.rs` containing 8 functions extracted from `agent_tick/mod.rs`: `enqueue_valid_step_or_handle_failure`, `finalize_agent_tick`, `resolve_step_targets`, `committed_action_for_step`, `apply_step_materialization_bindings`, `persist_blocked_memory`, `current_step`, `plan_finished`. Updated `mod.rs` with `mod execution;` and explicit re-exports. Cleaned up unused imports in `mod.rs`.
- **Deviations**: `handle_recoverable_travel_step_blockage` was made `pub(super)` (from private) so `execution.rs` could call it — minimal visibility change consistent with the sub-module pattern. `resolve_step_targets` is imported via `super::execution::resolve_step_targets` in tests rather than re-exported from `mod.rs`, to avoid an unused-import warning in non-test builds.
- **Verification**: `cargo test -p worldwake-ai` — 21/21 pass. `cargo clippy -p worldwake-ai` — zero warnings.
