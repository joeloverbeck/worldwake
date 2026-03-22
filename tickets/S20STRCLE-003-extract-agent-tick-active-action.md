# S20STRCLE-003: Extract agent_tick/active_action.rs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S20STRCLE-001

## Problem

The active-action handling logic in `agent_tick/mod.rs` (~300 lines) evaluates whether a running action should continue, be interrupted, or has completed. Extracting it into a dedicated sub-module clarifies the "act on current action" phase of the pipeline.

## Assumption Reassessment (2026-03-22)

1. Functions to extract (verified via grep):
   - `handle_active_action_phase()` (line 906)
   - `active_action_for_agent()` (line 669)
   - `effective_goal_switch_margin()` (line 993)
   - `goal_switch_margin_details()` (line 1002)
   - `advance_completed_step()` (line 1665)
   - `handle_current_step_failure()` (line 1715)
2. All are `fn` (private) or `pub(crate)` — no public API surface changes.
3. N/A — not planner/golden driven.
4. N/A — not an AI regression.
5. N/A — no ordering.
6. N/A — no heuristic changes.
7. N/A — not start-failure.
8. N/A — not political.
9. N/A — no ControlSource.
10. N/A — no golden scenario.
11. No mismatches.
12. N/A — no arithmetic.

## Architecture Check

1. These functions form a cohesive "evaluate and manage the currently active action" phase. They integrate interrupt evaluation, goal-switch margin calculations, and step advancement — a clear pipeline stage.
2. No backward-compatibility shims.

## Verification Layers

1. All tests pass → `cargo test -p worldwake-ai`.
2. Single-layer ticket: code motion only.

## What to Change

### 1. Create `agent_tick/active_action.rs`

Move the listed functions. Add necessary `use` imports.

### 2. Update `agent_tick/mod.rs`

- Add `mod active_action;`
- Add `use active_action::*;` or explicit re-imports.
- Remove moved function bodies.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/active_action.rs` (new)
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
