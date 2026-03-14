# E15RUMWITDIS-011: Integration Tests — Tell Propagation, Isolation, and Replay

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None expected — tests only unless a test exposes a real invariant gap
**Deps**: `archive/tickets/completed/E15RUMWITDIS-006.md`, `archive/tickets/completed/E15RUMWITDIS-008.md`, `archive/tickets/completed/E15RUMWITDIS-009.md`

## Problem

E15 already has strong unit coverage in `tell_actions.rs` and `perception.rs`, but the scheduler-level path is still under-tested. The remaining gap is not basic Tell or Discovery semantics in isolation; it is verifying that the full tick loop preserves locality and determinism when explicit action requests, travel time, perception updates, and replay all interact over multiple ticks.

## Assumption Reassessment (2026-03-14)

1. `replay_and_verify()` in `crates/worldwake-sim/src/replay_execution.rs` — the replay verification infrastructure exists and is used for determinism checks.
2. `SimulationState` in `crates/worldwake-sim/src/simulation_state.rs` — root state container for multi-tick simulation.
3. `step_tick()` in `crates/worldwake-sim/src/tick_step.rs` — executes one full tick.
4. Multi-tick integration harness patterns already exist in `crates/worldwake-systems/tests/e10_production_transport_integration.rs` and `crates/worldwake-systems/tests/e12_combat_integration.rs`.
5. `build_full_action_registries()` in `crates/worldwake-systems/src/action_registry.rs` already registers `tell` and `travel`, so new integration tests should use the production registry instead of hand-registering a partial action set.
6. Existing unit tests already cover most single-mechanic E15 behavior:
   - `crates/worldwake-systems/src/tell_actions.rs` covers source degradation, acceptance fidelity, memory capacity, relay limits, payload validation, and event tagging.
   - `crates/worldwake-systems/src/perception.rs` covers `Discovery` emission, `EntityMissing`, place-change mismatch, and `WitnessedTelling` social observations.
7. Current Tell architecture transmits `BelievedEntityState` about a `subject_entity`, not abstract event memories. Integration tests must therefore assert propagation of entity-state knowledge, not literal transmission of a prior event record.
8. The spec’s “tick 100 ... tick 112+” wording is an example of delayed propagation, not a hard architectural contract. Tests should assert relative timing from concrete action durations and topology, not magic absolute tick numbers.

## Architecture Check

1. These should remain test additions only unless execution exposes a real invariant bug.
2. The most valuable coverage is scheduler-level integration using the production action registry and dispatch table.
3. Tests should live in a dedicated `worldwake-systems/tests/` integration target rather than expanding already-large unit-test modules.
4. No backwards-compatibility shims.

## Scope Update

This ticket is narrowed to missing integration coverage that is still architecturally valuable:

1. `tell` propagation across travel and action duration in the real scheduler.
2. Isolation of unwitnessed events at remote or empty locations.
3. Bystander social observation isolation in the full tick loop.
4. Replay verification for a recorded E15 scenario through `replay_and_verify()`.

This ticket does **not** add a new event-memory system, rumor artifact model, or crime-specific report abstraction. If E15 later needs first-class “reports about events” rather than entity-state propagation, that should be a separate architecture change with new world-state carriers, not smuggled into tests.

## What to Change

### 1. Tell propagation delay integration test

Test the locality requirement using current architecture:

1. Set up world with two places connected by a travel edge whose duration is materially longer than Tell.
2. Agent A at place 1, Agent B at place 2.
3. Agent A starts with a concrete belief about subject entity S at place 1.
4. Agent A travels to place 2.
5. Verify Agent B remains ignorant while A is still remote.
6. After co-location, Agent A executes `tell` to Agent B.
7. Verify Agent B receives a degraded belief about S only after Tell completes.
8. Verify the assertion is relative:
   - no knowledge before physical meeting
   - no knowledge before Tell duration elapses
   - knowledge appears after Tell commit, not before

### 2. Hidden event at empty location test (T25 from spec)

1. Set up world with location X where no agents are present.
2. An event occurs at location X (no witnesses).
3. Agent at remote location Y remains ignorant — has no belief about the event.
4. Verify the remote agent’s belief store and social observation store remain unchanged.

### 3. Deterministic replay test

1. Run a multi-tick simulation with recorded E15-relevant inputs using the production scheduler.
2. Record initial state, seed, inputs, and checkpoints.
3. Replay from the same initial state and seed.
4. Verify `replay_and_verify()` succeeds for the scenario.
5. Prefer a scenario that includes at least one Tell commit; include Discovery only if it fits cleanly without building a fake architecture around event-memory transfer.

### 4. Bystander observation test

1. Three agents (A, B, C) at same place.
2. Agent A tells Agent B about subject S through the scheduler.
3. Verify Agent C records `WitnessedTelling`.
4. Verify Agent B receives the subject belief and Agent C does NOT.

## Files to Touch

- `crates/worldwake-systems/tests/` (new integration test file preferred)

## Out of Scope

- New production features or mechanics unless a test exposes a correctness bug
- AI tell-goal generation
- Event-memory/report abstractions beyond current `BelievedEntityState` Tell design
- Performance optimization

## Acceptance Criteria

### Tests That Must Pass

1. T25: Hidden event at empty location, no witnesses, remote agent remains ignorant
2. Information propagation delay: entity-state knowledge does not teleport; it arrives only after travel plus Tell completion
3. Deterministic replay: recorded E15 integration scenario passes `replay_and_verify()`
4. Bystanders observe `WitnessedTelling` but do NOT receive subject belief content
5. Existing suite: `cargo test --workspace`
6. `cargo clippy --workspace`

### Invariants

1. No information teleportation — entity beliefs only transfer through Tell or direct observation
2. Replay determinism remains intact for E15 action flows
3. Append-only event log integrity preserved

## Test Plan

### New/Modified Tests

1. Integration test for propagation delay — multi-tick travel + Tell through production scheduler
2. Integration test for T25 — unwitnessed remote event isolation
3. Integration test for deterministic replay — `replay_and_verify()` over recorded E15 scenario
4. Integration test for bystander isolation — `WitnessedTelling` without subject-belief leakage

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-14
- **What actually changed**:
  - Corrected the ticket assumptions to match current code and archived dependency locations.
  - Narrowed the scope from abstract event-memory propagation to the current E15 architecture: entity-belief propagation via `Tell`, bystander social observation isolation, unwitnessed-event isolation, and replay verification through the production scheduler.
  - Added a dedicated integration test target at `crates/worldwake-systems/tests/e15_information_integration.rs`.
- **Deviations from original plan**:
  - Kept production code unchanged.
  - Replaced the hard-coded `tick 100` / `tick 112+` framing with relative timing assertions tied to travel and Tell completion.
  - The replay test verifies a recorded Tell-plus-Discovery flow that is valid under the existing belief-only architecture; it does not add first-class event-memory transmission.
- **Verification results**:
  - `cargo test -p worldwake-systems` ✅
  - `cargo clippy --workspace` ✅
  - `cargo test --workspace` ✅
