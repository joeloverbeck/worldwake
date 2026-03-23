# S20STRCLE-004: Extract agent_tick/planning.rs

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S20STRCLE-001

## Problem

The planning orchestration logic in `agent_tick/mod.rs` (~400 lines) handles candidate plan building, plan search, validation, and selection. Extracting it into a dedicated sub-module clarifies the "decide what to do next" phase of the pipeline.

## Assumption Reassessment (2026-03-22)

1. Functions to extract (verified via grep):
   - `plan_and_validate_next_step()` (line 1115)
   - `plan_and_validate_next_step_traced()` (line 1239)
   - `build_candidate_plans()` (line 1024)
   - `plans_as_options()` (line 1100)
   - `plan_search_result_to_trace()` (line 1475)
   - `is_snapshot_changed_only()` (line 1228)
   - `summarize_step()` (line 472)
   - `summarize_selected_plan()` (line 492)
   - `summarize_search_provenance()` (line 514)
   - `summarize_plan_replacement()` (line 537)
   - `summarize_ranked_goal()` (line 565)
   - `determine_selected_plan_source()` (line 574)
2. All private or `pub(crate)` — no public API surface changes.
3. N/A — not planner/golden driven.
4. N/A — not an AI regression.
5. N/A — no ordering.
6. N/A — no heuristic changes.
7–12. N/A.

## Architecture Check

1. These functions form the "planning and selection" phase: generating candidate plans, running search, tracing, and selecting the best plan. Clear cohesion around "what should the agent do next?"
2. No backward-compatibility shims.

## Verification Layers

1. All tests pass → `cargo test -p worldwake-ai`.
2. Single-layer ticket: code motion only.

## What to Change

### 1. Create `agent_tick/planning.rs`

Move the listed functions. Add necessary `use` imports.

### 2. Update `agent_tick/mod.rs`

- Add `mod planning;`
- Add `use planning::*;` or explicit items.
- Remove moved function bodies.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (new)
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
- **What changed**: Extracted 12 planning-related functions from `agent_tick/mod.rs` into new `agent_tick/planning.rs`. Updated `mod.rs` imports and test module imports accordingly. Cleaned up unused imports that moved with the functions.
- **Deviations**: None. All 12 functions moved as specified. Line numbers in the ticket were stale (shifted by prior S20STRCLE-001/002/003 work) but function names and signatures matched exactly.
- **Verification**: `cargo test -p worldwake-ai` — 21 tests pass (unchanged from baseline). `cargo clippy -p worldwake-ai` — zero warnings.
