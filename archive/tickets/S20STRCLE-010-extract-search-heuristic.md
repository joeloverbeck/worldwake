# S20STRCLE-010: Extract search/heuristic.rs

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S20STRCLE-008

## Problem

The heuristic computation logic in `search/mod.rs` (~200 lines) handles A* heuristic calculation, Dijkstra integration, and relevant-place computation. Extracting it into a dedicated sub-module isolates the cost-estimation concern.

## Assumption Reassessment (2026-03-22)

1. Functions and types to extract (verified via grep):
   - `compute_heuristic()` (line 323)
   - `CombinedRelevantPlaces` struct (line 338)
   - `combined_relevant_places()` (line 344)
   - `root_node()` (line 367)
   - `prune_travel_away_from_goal()` (line 391)
2. All private — no public API surface changes.
3–12. N/A — pure structural refactor.

## Architecture Check

1. Heuristic computation is a self-contained mathematical concern: distance estimation, relevant-place identification, and travel pruning. Isolating it makes the A* cost model independently reviewable.
2. No backward-compatibility shims.

## Verification Layers

1. All tests pass → `cargo test -p worldwake-ai`.
2. Single-layer ticket: code motion only.

## What to Change

### 1. Create `search/heuristic.rs`

Move the listed functions and structs. Import `SearchNode` and other shared types from `super`.

### 2. Update `search/mod.rs`

- Add `mod heuristic;`
- Add `use heuristic::*;`.
- Remove moved items.

## Files to Touch

- `crates/worldwake-ai/src/search/heuristic.rs` (new)
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
- **What changed**: Extracted `compute_heuristic()`, `CombinedRelevantPlaces`, `combined_relevant_places()`, `root_node()`, and `prune_travel_away_from_goal()` from `search/mod.rs` into new `search/heuristic.rs` with `pub(super)` visibility. Updated `mod.rs` with `mod heuristic;` and explicit use import. Removed stale `use crate::goal_model::trace_prerequisite_guidance;` import from `mod.rs`.
- **Deviations**: Ticket suggested `use heuristic::*;` but clippy's `wildcard_imports` lint (enabled via `pedantic`) required explicit imports instead.
- **Verification**: `cargo test -p worldwake-ai` — 21 tests pass. `cargo clippy -p worldwake-ai` — clean.
