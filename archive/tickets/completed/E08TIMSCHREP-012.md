# E08TIMSCHREP-012: Phase 1 gate coverage audit for E08 replay/save-load

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: No
**Deps**: E08TIMSCHREP-009 (replay execution), E08TIMSCHREP-010 (simulation state), E08TIMSCHREP-011 (save/load)
**Spec Ref**: `archive/specs/E08-time-scheduler-replay.corrected.md` ("State Equality / Test Utilities", "Tests", "Phase 1 Gate")

## Problem

The original ticket assumed E08 still lacked state-equality helpers, reusable tick-driving helpers, and a dedicated Phase 1 gate integration test file. That assumption is no longer true. Before adding more infrastructure, this ticket needed to reassess the actual code and test architecture so we did not introduce redundant wrappers or split the gate proof across an unnecessary second test harness.

## Assumption Reassessment (2026-03-09)

1. `SimulationState` already exists, is `Eq`, and already exposes canonical hashing through `SimulationState::hash()` and `SimulationState::replay_bootstrap_hash()` — confirmed.
2. The proposed `assert_states_equal(...)` helper would only wrap `assert_eq!` on an already-structured root type and would not improve the architecture — confirmed.
3. There is no `SystemRegistry` in the current architecture. Tick execution is driven by `step_tick(...)` plus `TickStepServices<'_>` and `SystemDispatchTable` — confirmed.
4. Replay determinism proof utilities already exist in `crates/worldwake-sim/src/replay_execution.rs`: `replay_and_verify(...)`, `record_tick_checkpoint(...)`, and `seed_replay_inputs_from_scheduler(...)` — confirmed.
5. Save/load already includes the strongest semantic proof this ticket wanted: `loaded_state_continues_identically_to_uninterrupted_execution` in `crates/worldwake-sim/src/save_load.rs` — confirmed.
6. Phase 1 gate coverage is intentionally distributed across the owning modules and crates, not centralized in a single `phase1_gate.rs` file:
   - T01, T04, T13: `crates/worldwake-core/tests/relation_invariants.rs`
   - T02: `crates/worldwake-core/src/conservation.rs`
   - T05: `crates/worldwake-sim/src/start_gate.rs` and `crates/worldwake-sim/src/affordance_query.rs`
   - T06: `crates/worldwake-sim/src/tick_action.rs`
   - T07: `crates/worldwake-core/src/verification.rs`
   - T08: `crates/worldwake-sim/src/replay_execution.rs`
   - T09: `crates/worldwake-sim/src/save_load.rs`
7. The randomized Phase 1 invariant sweep already exists where the underlying invariants live today. The current codebase does not justify a second cross-crate gate harness that would mostly duplicate setup and assertions — confirmed.

## Architecture Check

1. The existing architecture is cleaner than the ticket's original proposal. Exact state equality is a property of `SimulationState`, not a separate utility layer.
2. Replay and save/load proofs belong beside the production code they validate. Keeping those tests in `replay_execution.rs` and `save_load.rs` avoids an artificial integration layer that would drift from the real APIs.
3. A new public `worldwake-sim::test_utils` module would be premature. Today it would mostly alias existing operations (`assert_eq!`, `step_tick`, `replay_and_verify`, `save_to_bytes`/`load_from_bytes`) without adding new domain behavior.
4. A single umbrella `phase1_gate.rs` integration test is not currently more robust than the distributed suite. The current ownership boundaries match the foundational principle that systems and invariants should be proven close to the authoritative state they govern.

## Scope Correction

This ticket should:

1. Audit the E08 assumptions against the current implementation.
2. Confirm whether any real Phase 1 gate gap remains after E08TIMSCHREP-009 through E08TIMSCHREP-011.
3. Record the actual test ownership and verification matrix.
4. Close the ticket if no additional implementation is justified.

This ticket should not:

1. Introduce a new `test_utils` facade just to wrap `assert_eq!`, `step_tick(...)`, or existing replay/save-load helpers.
2. Add a duplicate `phase1_gate.rs` integration file when the current gate proofs already exist in the owning modules.
3. Re-test older Phase 1 invariants by re-implementing their assertions inside `worldwake-sim`.

## Proposed Changes Reassessed

### `assert_states_equal`

Not beneficial. `SimulationState` already derives `Eq`, so `assert_eq!(left, right)` is the canonical exact-state proof. Adding a named wrapper would hide no complexity and would add another surface to maintain.

### `assert_hashes_equal`

Not beneficial as a shared API. Existing replay/save-load tests already compare hashes directly at the call sites that know the expected meaning of each hash. A wrapper would not improve clarity.

### `run_ticks`

Not beneficial as a public reusable helper today. The real tick boundary is `step_tick(...)` with a full `TickStepServices<'_>` bundle. A generic wrapper would either duplicate this surface or erase important setup detail.

### `run_and_compare_with_save_load`

Already effectively implemented by the current save/load continuation test. If this comparison pattern starts repeating in multiple modules later, a private support helper could be introduced then. It is not justified now.

### New `phase1_gate.rs` integration test

Not beneficial under the current architecture. The authoritative gate coverage already exists across `worldwake-core` and `worldwake-sim`, and the workspace test run is the gate-level aggregation.

## Files Touched

- `tickets/E08TIMSCHREP-012.md` (updated and completed)

## Out of Scope

- Adding new replay/save-load code paths
- Moving existing inline tests into integration tests
- Refactoring the current gate test layout

## Acceptance Criteria

1. Ticket assumptions are corrected to match the actual `worldwake-sim` and `worldwake-core` architecture.
2. The ticket records where T01-T09 and T13 are actually enforced.
3. Verification proves the existing suites are green without adding redundant abstraction layers.
4. The ticket is archived with an outcome section once the audit is complete.

## Test Plan

### New/Modified Tests

None. The audit found the existing test architecture already covers the originally proposed scope.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-core --test relation_invariants`
3. `cargo test -p worldwake-core verification`
4. `cargo test -p worldwake-core conservation`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`

## Outcome

Completion date: 2026-03-09

What actually changed:

1. Reassessed the ticket against the implemented E08 architecture and corrected its assumptions.
2. Confirmed that replay determinism, save/load semantic continuation, exact-state equality, and Phase 1 gate coverage already exist in the owning modules.
3. Kept the architecture as-is instead of adding a redundant `test_utils` facade or duplicate gate integration file.

Differences from the original plan:

1. No new `crates/worldwake-sim/src/test_utils.rs` was added because it would mostly alias existing APIs.
2. No `crates/worldwake-sim/tests/phase1_gate.rs` file was added because the gate proofs already exist in the authoritative modules and earlier crate-level test suites.
3. No new equality/hash assertion wrappers were introduced because `SimulationState` equality and existing direct hash assertions are already the cleaner architecture.

Verification results:

1. `cargo test -p worldwake-sim` passed.
2. `cargo test -p worldwake-core --test relation_invariants` passed.
3. `cargo test -p worldwake-core verification` passed.
4. `cargo test -p worldwake-core conservation` passed.
5. `cargo clippy --workspace --all-targets -- -D warnings` passed.
6. `cargo test --workspace` passed.
