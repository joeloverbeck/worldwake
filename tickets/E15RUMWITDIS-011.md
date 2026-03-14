# E15RUMWITDIS-011: Integration Tests — Information Propagation, Replay, and Isolation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: E15RUMWITDIS-006 (Tell handler), E15RUMWITDIS-008 (mismatch detection), E15RUMWITDIS-009 (EntityMissing)

## Problem

The E15 spec defines several integration-level acceptance tests that span multiple systems and ticks: information propagation delay (crime at tick 100, witness travels, tells at tick 112+), hidden events at empty locations where remote agents remain ignorant, and deterministic replay of Tell actions and Discovery events. These cannot be tested in isolation and require multi-tick simulation with the full action framework.

## Assumption Reassessment (2026-03-14)

1. `replay_and_verify()` in `crates/worldwake-sim/src/replay_execution.rs` — the replay verification infrastructure exists and is used for determinism checks.
2. `SimulationState` in `crates/worldwake-sim/src/simulation_state.rs` — root state container for multi-tick simulation.
3. `step_tick()` in `crates/worldwake-sim/src/tick_step.rs` — executes one full tick.
4. The test pattern for multi-tick scenarios is established in existing integration tests across the crates.

## Architecture Check

1. These are pure test additions — no production code changes.
2. Tests exercise the full simulation stack: action framework + perception + Tell + mismatch detection + replay.
3. Tests go in a dedicated integration test file in worldwake-systems or a cross-crate test.
4. No backwards-compatibility shims.

## What to Change

### 1. Information propagation delay test

Test the spec requirement: "crime at tick 100, witness travels, tells at tick 112+"

1. Set up world with two places connected by a travel edge (travel time ≥ 10 ticks).
2. Agent A at place 1, Agent B at place 2.
3. At tick 100, an event occurs at place 1 (e.g., combat event). Agent A witnesses it.
4. Agent A begins traveling to place 2.
5. Agent A arrives at place 2 (tick 110+).
6. Agent A executes Tell to Agent B (2 ticks).
7. Verify Agent B's belief about the event is updated at tick 112+.
8. Verify Agent B did NOT have any knowledge before the Tell.

### 2. Hidden event at empty location test (T25 from spec)

1. Set up world with location X where no agents are present.
2. An event occurs at location X (no witnesses).
3. Agent at remote location Y remains ignorant — has no belief about the event.
4. Verify Agent's belief store is unchanged.

### 3. Deterministic replay test

1. Run a multi-tick simulation with Tell actions and Discovery events.
2. Record initial state, seed, and inputs.
3. Replay from the same initial state and seed.
4. Verify all Tell events and Discovery events reproduce identically (same ticks, same evidence, same belief stores).

### 4. Bystander observation test

1. Three agents (A, B, C) at same place.
2. Agent A tells Agent B about subject S.
3. Verify Agent C observes WitnessedTelling social observation.
4. Verify Agent C does NOT receive the belief content about S.

## Files to Touch

- `crates/worldwake-systems/src/tell_actions.rs` or `crates/worldwake-systems/tests/` (new/modify — integration tests)

## Out of Scope

- Production code changes
- AI goal generation
- New features or mechanics
- Performance optimization

## Acceptance Criteria

### Tests That Must Pass

1. T25: Hidden event at empty location, no witnesses, remote agent remains ignorant
2. Information propagation delay: crime at tick 100, witness travels, tells at tick 112+ (not before)
3. Deterministic replay: Tell actions and Discovery events reproduce identically
4. Bystanders observe WitnessedTelling but do NOT receive belief content
5. Existing suite: `cargo test --workspace`
6. `cargo clippy --workspace`

### Invariants

1. No information teleportation — beliefs only transfer through Tell or direct observation
2. Replay determinism maintained with new action types
3. Append-only event log integrity preserved

## Test Plan

### New/Modified Tests

1. Integration test for information propagation delay — multi-tick simulation with travel + Tell
2. Integration test for T25 — hidden event isolation
3. Integration test for deterministic replay — full replay_and_verify with Tell
4. Integration test for bystander isolation — WitnessedTelling vs belief content separation

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
