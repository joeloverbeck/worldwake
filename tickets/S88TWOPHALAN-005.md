# S88TWOPHALAN-005: Implement landmark count heuristic

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — new function in existing heuristic module
**Deps**: S88TWOPHALAN-003

## Problem

The planner's current heuristic is spatial-only (minimum travel distance to goal-relevant places). It provides no guidance about subgoal structure — an agent may be at the right place but still far from satisfying the goal. The landmark count heuristic (S88 D7) counts unachieved actionable landmarks, providing complementary structure guidance that the spatial heuristic cannot.

## Assumption Reassessment (2026-04-11)

1. `compute_heuristic()` exists at `crates/worldwake-ai/src/search/heuristic.rs:17`. It returns `u32` representing minimum perceived travel cost. Called from `root_node()` at line 119 and from `build_successor_detailed` in `transition.rs`.
2. `LandmarkSet` will be defined by S88TWOPHALAN-003 in `search/landmarks.rs`. This ticket depends on that type existing.
3. The spec's combined heuristic uses `spatial_h.max(landmark_h)` — taking the max preserves both guidance properties within the existing satisficing search.

## Architecture Check

1. A standalone pure function `compute_landmark_heuristic(landmarks, current_facts) -> u32` in the existing `heuristic.rs` module is the cleanest approach — it follows the same pattern as `compute_heuristic()`.
2. No backwards-compatibility shims. The combined heuristic will be wired in S88TWOPHALAN-007.

## Verification Layers

1. Heuristic value correctness → focused unit tests (counts unachieved actionable landmarks)
2. Returns 0 when all landmarks achieved → focused unit test
3. Ignores landmarks whose predecessors are not yet achieved → focused unit test
4. Single-layer ticket (planner-internal heuristic function) — no cross-layer mapping needed.

## What to Change

### 1. Add `compute_landmark_heuristic` to `crates/worldwake-ai/src/search/heuristic.rs`

```rust
pub(super) fn compute_landmark_heuristic(
    landmarks: &LandmarkSet,
    current_facts: &BTreeSet<PlanningFact>,
) -> u32
```

Returns the count of landmarks in `landmarks.landmarks` that:
- Are NOT yet achieved in `current_facts`
- Have all ordering predecessors (from `landmarks.orderings`) achieved in `current_facts`

This counts "how many mandatory milestones are currently actionable but unachieved."

### 2. Write focused unit tests

- `test_landmark_heuristic_all_achieved` — all landmarks in current_facts → returns 0
- `test_landmark_heuristic_counts_actionable` — landmarks whose predecessors are achieved but which are not in current_facts → counted
- `test_landmark_heuristic_ignores_blocked` — landmarks with unachieved predecessors → not counted
- `test_landmark_heuristic_empty_landmarks` — empty LandmarkSet → returns 0

## Files to Touch

- `crates/worldwake-ai/src/search/heuristic.rs` (modify)

## Out of Scope

- Combining spatial + landmark heuristic in `search_plan()` (S88TWOPHALAN-007)
- Landmark extraction itself (S88TWOPHALAN-003)
- Modifying how heuristic values affect frontier ordering

## Acceptance Criteria

### Tests That Must Pass

1. All 4 focused unit tests for landmark count heuristic
2. Existing suite: `cargo test -p worldwake-ai -- heuristic`

### Invariants

1. Returns 0 when `landmarks` is empty
2. Returns 0 when all landmarks are in `current_facts`
3. Never counts landmarks whose ordering predecessors are not all achieved

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/heuristic.rs` (inline tests) — landmark count heuristic correctness

### Commands

1. `cargo test -p worldwake-ai -- heuristic`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
