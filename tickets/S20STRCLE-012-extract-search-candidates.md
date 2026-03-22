# S20STRCLE-012: Extract search/candidates.rs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S20STRCLE-008

## Problem

The search candidate generation logic in `search/mod.rs` (~500 lines) handles action-def filtering, binding rejection, travel pruning, facility exclusivity, and candidate construction. Extracting it into a dedicated sub-module isolates "which actions can the agent try at this search node?" from the main search loop.

## Assumption Reassessment (2026-03-22)

1. Functions to extract (verified via grep):
   - `search_candidates()` (line 671)
   - `relevant_action_defs()` (line 585)
   - `root_candidate_payload_error()` (line 597)
   - `push_root_candidate_trace()` (line 613)
   - `update_root_candidate_outcome()` (line 622)
   - `root_candidate_payload_status()` (line 638)
   - `root_candidate_trace_from_candidate()` (line 651)
   - `candidate_blocked_facility_use()` (line 761)
   - `intended_exclusive_action()` (line 774)
   - `search_candidates_from_affordance()` (line 794)
   - `queue_intended_actions_for()` (line 865)
   - `search_candidate_from_planner()` (line 907)
   - `unsupported_goal()` (line 920)
   - `SearchCandidate` struct (line 34)
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
