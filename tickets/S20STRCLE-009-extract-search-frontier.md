# S20STRCLE-009: Extract search/frontier.rs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S20STRCLE-008

## Problem

The frontier/priority-queue management logic in `search/mod.rs` (~300 lines) handles search node comparison, beam-width truncation, and binary heap management. Extracting it into a dedicated sub-module separates data-structure mechanics from search semantics.

## Assumption Reassessment (2026-03-22)

1. Types and functions to extract (verified via grep):
   - `FrontierEntry` struct (line 29) + `PartialEq`, `Eq`, `PartialOrd`, `Ord` impls (lines 53–75)
   - `compare_search_nodes()` (line 924)
   - `SearchNode` struct (line 18) — shared across modules, may stay in `mod.rs` or move here if it's primarily frontier-owned. Decision: keep `SearchNode` in `mod.rs` since it's used by all sub-modules.
2. All private — no public API surface changes.
3–12. N/A — pure structural refactor.

## Architecture Check

1. Frontier management is a self-contained algorithmic concern: ordering, truncation, comparison. Separating it from search semantics clarifies which code is "data structure mechanics" vs "domain logic."
2. No backward-compatibility shims.

## Verification Layers

1. All tests pass → `cargo test -p worldwake-ai`.
2. Single-layer ticket: code motion only.

## What to Change

### 1. Create `search/frontier.rs`

Move `FrontierEntry`, its trait impls, and `compare_search_nodes()`. Import `SearchNode` from `super`.

### 2. Update `search/mod.rs`

- Add `mod frontier;`
- Add `use frontier::*;`.
- Remove moved items.

## Files to Touch

- `crates/worldwake-ai/src/search/frontier.rs` (new)
- `crates/worldwake-ai/src/search/mod.rs` (modify)

## Out of Scope

- Extracting other search sub-modules (heuristic, transition, candidates)
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
2. `lib.rs` re-exports `search_plan`, `PlanSearchResult` continue to resolve
3. Function signatures and visibility unchanged

## Test Plan

### New/Modified Tests

1. None — code motion only; verification is command-based.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
