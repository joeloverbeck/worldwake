# E08TIMSCHREP-012: State equality utilities and Phase 1 gate tests

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — integration test infrastructure in worldwake-sim
**Deps**: E08TIMSCHREP-009 (replay execution), E08TIMSCHREP-011 (save/load)

## Problem

The E08 spec requires test utilities for exact final-state equality, checkpoint hash comparisons, and uninterrupted-vs-save/load-continued comparisons. These utilities power the Phase 1 gate tests (T01–T09, T13). Without them, the gate cannot be verified.

## Assumption Reassessment (2026-03-09)

1. All prior E08 tickets provide the building blocks: `SimulationState`, `StateHash`, `save`/`load`, `replay_and_verify`, `step_tick`
2. Phase 1 gate tests T01–T07 and T13 were established in E01–E07 — they must still pass after E08 changes
3. T08 (replay determinism) is primarily tested in E08TIMSCHREP-009 but needs a gate-level integration test here
4. T09 (save/load round-trip) is primarily tested in E08TIMSCHREP-011 but needs a gate-level integration test here

## Architecture Check

1. Test utilities live in a `test_utils` submodule of worldwake-sim (or extend the existing `worldwake-core::test_utils`)
2. Gate tests are integration tests in `crates/worldwake-sim/tests/` — they exercise the full stack
3. The "uninterrupted vs save/load continued" test is the strongest correctness proof: run N ticks → save → load → run M more ticks → compare against running N+M ticks uninterrupted

## What to Change

### 1. Test utility: `assert_states_equal`

```rust
pub fn assert_states_equal(a: &SimulationState, b: &SimulationState) {
    assert_eq!(a, b, "SimulationState mismatch");
}
```

Plus a more diagnostic version that reports which field differs first.

### 2. Test utility: `assert_hashes_equal`

```rust
pub fn assert_hashes_equal(label: &str, expected: StateHash, actual: StateHash) {
    assert_eq!(expected, actual, "{label}: hash mismatch");
}
```

### 3. Test utility: `run_ticks`

```rust
pub fn run_ticks(
    state: &mut SimulationState,
    systems: &SystemRegistry,
    n: u64,
) -> Vec<TickStepResult>
```

Convenience wrapper that calls `step_tick` N times.

### 4. Test utility: `run_and_compare_with_save_load`

```rust
pub fn run_and_compare_with_save_load(
    initial: &SimulationState,
    systems: &SystemRegistry,
    split_at_tick: u64,
    total_ticks: u64,
) -> Result<(), String>
```

Runs `total_ticks` uninterrupted, then separately runs `split_at_tick`, saves, loads, runs remainder — asserts final states are equal.

### 5. Phase 1 gate integration tests

Integration test file that runs:
- T08: replay determinism — multi-seed, multi-input replay test
- T09: save/load round-trip — semantic continuation test
- Verify T01–T07 and T13 still pass (reference existing tests, ensure E08 changes don't break them)
- "Randomized invariant tests across E01–E08" — run random inputs for 50+ ticks, verify conservation, unique location, acyclic containment, event provenance after each tick

## Files to Touch

- `crates/worldwake-sim/src/test_utils.rs` (new — `#[cfg(test)]` or `pub` for integration tests)
- `crates/worldwake-sim/tests/phase1_gate.rs` (new — integration test)
- `crates/worldwake-sim/src/lib.rs` (modify — add test_utils module)

## Out of Scope

- Fixing bugs found by gate tests (separate bug-fix tickets)
- Phase 2 gate tests (E09–E13)
- Performance benchmarks
- CLI integration for running gate tests

## Acceptance Criteria

### Tests That Must Pass

1. `assert_states_equal` passes for identical states, panics for different states
2. `run_ticks` produces deterministic results (same state twice = same output)
3. `run_and_compare_with_save_load` passes: uninterrupted run matches save/load/continue run
4. **T08 gate test**: 3+ different seeds, 10+ ticks each with inputs → replay matches
5. **T09 gate test**: save at tick 5, load, continue to tick 15 → matches running 15 ticks straight
6. **T01–T07, T13 regression**: all pre-existing Phase 1 invariant tests still pass
7. **Randomized invariant sweep**: 50-tick run with random inputs, verify conservation + unique location + acyclic containment + event provenance hold after every tick
8. Existing suite: `cargo test --workspace`

### Invariants

1. All Phase 1 gate invariants (T01–T09, T13) remain green
2. No test utilities introduce nondeterminism
3. Test utilities are reusable by Phase 2+ tests

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/test_utils.rs` — utility function tests
2. `crates/worldwake-sim/tests/phase1_gate.rs` — full gate integration tests

### Commands

1. `cargo test -p worldwake-sim phase1_gate`
2. `cargo test -p worldwake-sim test_utils`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
