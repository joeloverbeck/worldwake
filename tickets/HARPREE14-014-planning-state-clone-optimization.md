# HARPREE14-014: Reduce PlanningState clone overhead (OPTIONAL)

**Status**: PENDING
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: Yes -- data structure change in PlanningState (if implemented)
**Deps**: None (Wave 5, optional)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-C03

## Problem

`PlanningState` is cloned on every search node expansion. The override maps (`BTreeMap`) inside are cloned deeply. At larger beam widths this could become expensive. Currently the project uses small beam widths, so this is a future-proofing concern.

## Assumption Reassessment (2026-03-11)

1. `PlanningState` contains 9 BTreeMap override maps -- confirmed (entity_place, direct_container, direct_possessor, resource_quantity, commodity_quantity, reservation_shadows, needs_overrides, pain_overrides, plus removed_entities)
2. Cloning happens on every search node expansion -- confirmed
3. Current beam width is small, so this is not a current bottleneck -- confirmed

## Architecture Check

1. `im::OrdMap` provides structural sharing (O(1) clone, O(log n) update) -- standard approach for this pattern.
2. This is the ONLY ticket that may add an external dependency (`im` crate).
3. If benchmarks show <20% improvement, document findings and close without implementing.

## What to Change

### 1. Benchmark baseline

Measure clone cost at beam_width=8 vs beam_width=32 with the current BTreeMap implementation.

### 2. Investigate `im::OrdMap` or cow-style sharing

Replace `BTreeMap` fields in `PlanningState` with `im::OrdMap` (or implement a custom cow wrapper).

### 3. Benchmark improvement

Measure clone cost with the new implementation. If improvement is >20%, keep. Otherwise, revert and document findings.

### 4. Document findings either way

Whether implemented or not, document the benchmark results for future reference.

## Files to Touch

- `crates/worldwake-ai/src/planning_state.rs` (modify -- if implementing)
- `crates/worldwake-ai/Cargo.toml` (modify -- if adding `im` dependency)

## Out of Scope

- Changing PlanningState API or semantics
- Modifying search algorithm
- Changing other data structures in the AI crate
- Optimizations to non-clone operations

## Acceptance Criteria

### Tests That Must Pass

1. All search and planning tests pass unchanged
2. Golden e2e hashes identical (determinism preserved -- `im::OrdMap` uses `Ord` ordering)
3. `cargo test --workspace` passes
4. `cargo clippy --workspace` -- no new warnings
5. Benchmark results documented (commit message or doc comment)

### Invariants

1. PlanningState behavior unchanged (same results for same inputs)
2. Determinism preserved
3. Golden e2e state hashes identical
4. If `im` added, it's the ONLY new external dependency in the entire hardening effort

## Test Plan

### New/Modified Tests

1. No new behavior tests -- existing tests validate identical behavior
2. Optionally add a benchmark test for clone performance

### Commands

1. `cargo test -p worldwake-ai search` (targeted)
2. `cargo test -p worldwake-ai --test golden_e2e` (determinism check)
3. `cargo test --workspace`
4. `cargo clippy --workspace`
