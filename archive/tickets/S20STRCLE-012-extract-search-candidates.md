# S20STRCLE-012: Extract search/candidates.rs

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S20STRCLE-008

## Problem

The search candidate generation logic in `search/mod.rs` (~500 lines) handles action-def filtering, binding rejection, travel pruning, facility exclusivity, and candidate construction. Extracting it into a dedicated sub-module isolates "which actions can the agent try at this search node?" from the main search loop.

## Assumption Reassessment (2026-03-22)

1. Functions to extract (re-verified 2026-03-23, post S20STRCLE-008–011 extractions):
   - `SearchCandidate` struct (line 37)
   - `relevant_action_defs()` (line 291)
   - `push_root_candidate_trace()` (line 303)
   - `update_root_candidate_outcome()` (line 312)
   - `root_candidate_payload_status()` (line 328)
   - `root_candidate_trace_from_candidate()` (line 341)
   - `search_candidates()` (line 361)
   - `candidate_blocked_facility_use()` (line 451)
   - `intended_exclusive_action()` (line 464)
   - `search_candidates_from_affordance()` (line 484)
   - `queue_intended_actions_for()` (line 555)
   - `search_candidate_from_planner()` (line 597)
   - `unsupported_goal()` (line 610)
   - ~~`root_candidate_payload_error()`~~ — does not exist in current code; removed from scope
2. All private — no public API surface changes.
3–12. N/A — pure structural refactor.

## Architecture Check

1. Search candidate generation is the largest sub-concern in `search.rs`: it decides which actions to expand at each search node. Isolating it makes the main search loop (`search_plan`) dramatically shorter and more readable.
2. No backward-compatibility shims.

## Verification Layers

1. All tests pass → `cargo test -p worldwake-ai`.
2. Single-layer ticket: code motion only.

## What to Change

### 1. Create `search/candidates.rs`

Move the listed functions and `SearchCandidate` struct. Import `SearchNode` and other shared types from `super`.

### 2. Update `search/mod.rs`

- Add `mod candidates;`
- Add `use candidates::*;`.
- Remove moved items.

## Files to Touch

- `crates/worldwake-ai/src/search/candidates.rs` (new)
- `crates/worldwake-ai/src/search/mod.rs` (modify)

## Out of Scope

- Extracting other search sub-modules
- Changing function signatures, visibility, or logic
- Modifying `agent_tick/`
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
- **What changed**: Created `crates/worldwake-ai/src/search/candidates.rs` with 12 functions + `SearchCandidate` struct extracted from `search/mod.rs`. Updated `mod.rs` with explicit imports (`use candidates::{...}`) instead of wildcard (clippy pedantic enforces this).
- **Deviations**: `root_candidate_payload_error()` listed in original ticket does not exist in current code — removed from scope. Used explicit imports instead of `use candidates::*;` per clippy pedantic lint. Two test-only re-exports gated behind `#[cfg(test)]`.
- **Verification**: `cargo test -p worldwake-ai` — 21 tests passed. `cargo clippy -p worldwake-ai` — no warnings.
