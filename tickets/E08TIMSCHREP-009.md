# E08TIMSCHREP-009: Replay execution and verification

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: E08TIMSCHREP-006 (step_tick), E08TIMSCHREP-007 (canonical hashing), E08TIMSCHREP-008 (ReplayState)
**Spec Ref**: `specs/E08-time-scheduler-replay.corrected.md` ("Replay Recording", "Replay Execution", "Per-Tick Flow")

## Problem

The replay system must prove determinism: given the same initial state, same seed, and same input log, the simulation must produce identical checkpoint hashes and identical final state. This is the T08 gate test. Without a replay executor, determinism is a claim, not a proven property.

## Assumption Reassessment (2026-03-09)

1. `step_tick` already defines the authoritative tick boundary and already consumes a `TickStepServices` bundle containing action registries plus `SystemDispatchTable` — confirmed.
2. `ReplayState` now exists and stores the initial state hash, master seed, ordered input log, replay checkpoints, and checkpoint policy — confirmed.
3. `StateHash`, `hash_world`, and `hash_event_log` already exist in `worldwake-core` and are the correct canonical comparison primitives for replay checkpoints — confirmed.
4. `DeterministicRng` reconstructs from `Seed` and uses deterministic substreams per `(tick, system_id, seq)` — confirmed.
5. `Scheduler` owns the mutable `InputQueue`; replay should rebuild that queue from the recorded `ReplayState::input_log()` on a cloned scheduler, not invent a parallel execution path — confirmed.
6. There is no `SystemRegistry` type in the current architecture. Runtime systems are supplied through `SystemDispatchTable`, and replay must use that same interface to stay aligned with normal execution — confirmed.
7. There is no `SimulationState` root type yet. Replay execution should therefore operate on explicit state roots (`World`, `EventLog`, `Scheduler`, `ControllerState`) until E08TIMSCHREP-010 introduces a consolidated simulation root.

## Architecture Check

1. Replay must reuse the exact production tick path (`step_tick`) rather than a replay-specific simulation loop. Determinism proof is strongest when replay exercises the same scheduler, action, and system boundaries as live execution.
2. Replay should clone the provided authoritative roots and seed a fresh `DeterministicRng` from the recording's master seed. The original state objects must remain untouched.
3. The replay API should accept the same registries/dispatch objects that `step_tick` already needs. Introducing a separate replay-specific registry abstraction would be architectural duplication.
4. Checkpoint verification should compare recorded `ReplayCheckpoint` entries directly, in recorded order, and should report mismatches with concrete tick and checkpoint-kind data.
5. Recording helpers that compute and append checkpoints after a tick step are appropriate here because they are shared replay infrastructure and keep hashing/append rules out of `step_tick` itself.

## Scope Correction

This ticket should:

1. Implement replay execution and verification on top of the existing `step_tick` + `TickStepServices` architecture.
2. Add a shared helper for recording per-tick checkpoints into `ReplayState`.
3. Provide the T08 determinism proof tests for the current Phase 1 scheduler stack.

This ticket should not:

1. Introduce a shadow replay runner with different execution phases.
2. Depend on a not-yet-existing `SimulationState` root.
3. Rework scheduler ownership boundaries established in E08TIMSCHREP-006 and E08TIMSCHREP-008.

## What to Change

### 1. New type: `ReplayError`

```rust
pub enum ReplayError {
    InitialHashMismatch { expected: StateHash, actual: StateHash },
    CheckpointMismatch {
        tick: Tick,
        kind: CheckpointKind, // WorldState or EventLog
        expected: StateHash,
        actual: StateHash,
    },
    FinalHashMismatch { expected: StateHash, actual: StateHash },
    TickCountMismatch { expected: u64, actual: u64 },
}

pub enum CheckpointKind {
    WorldState,
    EventLog,
}
```

### 2. New function: `replay_and_verify`

```rust
pub fn replay_and_verify(
    initial_world: &World,
    initial_event_log: &EventLog,
    initial_scheduler: &Scheduler,
    initial_controller: &ControllerState,
    replay: &ReplayState,
    services: TickStepServices<'_>,
) -> Result<StateHash, Vec<ReplayError>>
```

Flow:
1. Reconstruct `DeterministicRng` from `replay.master_seed()`
2. Verify initial state hash matches `replay.initial_state_hash()`
3. Clone `World`, `EventLog`, `Scheduler`, and `ControllerState`
4. Populate the cloned scheduler's `InputQueue` from `replay.input_log()`
5. Step through ticks with `step_tick(...)`, reusing the provided `TickStepServices`
6. After each stepped tick, compare any expected checkpoint for that tick against freshly computed hashes
7. After replay reaches the last recorded checkpoint/input boundary, compute and return the final world-state hash
8. Return the final hash on success, or accumulated replay errors

### 3. Integration with recording

Add a helper that computes and appends a replay checkpoint after a completed tick when the recording policy says that tick should be checkpointed:

```rust
pub fn record_tick_checkpoint(
    replay: &mut ReplayState,
    tick: Tick,
    world: &World,
    event_log: &EventLog,
) -> Result<bool, ReplayCheckpointError>
```

Behavior:

1. If `replay.should_checkpoint(tick)` is `false`, do nothing and return `Ok(false)`.
2. Otherwise compute `hash_event_log(event_log)` and `hash_world(world)`, append a `ReplayCheckpoint`, and return `Ok(true)`.
3. Surface either canonical-hashing failures or replay-state append failures through a small replay-checkpoint error type.

## Files to Touch

- `crates/worldwake-sim/src/replay_execution.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Save/load of replay recordings to disk (E08TIMSCHREP-011 handles persistence)
- Streaming replay (play-back at real-time speed for visualization)
- Replay UI/CLI (E21)
- Concrete game systems (E09–E12) — tests use mock/no-op systems

## Acceptance Criteria

### Tests That Must Pass

1. **T08 — Replay determinism**: Run 10+ ticks with inputs → record → replay from initial state → all checkpoint hashes match and final hash matches
2. Replay with zero inputs: empty simulation replays identically
3. Replay with multiple same-tick inputs: ordering preserved, identical result
4. Replay detects initial hash mismatch when given wrong initial state
5. Replay detects checkpoint mismatch when state is corrupted mid-replay (inject a mutation)
6. `ReplayError` variants are descriptive (tick, expected/actual hashes)
7. `record_tick_checkpoint` records nothing when `ReplayRecordingConfig::disabled()` is used
8. Replay with `ReplayRecordingConfig::every(NonZeroU64::new(1).unwrap())`: every tick verified
9. Replay with `ReplayRecordingConfig::every(NonZeroU64::new(5).unwrap())`: only ticks 0, 5, 10, ... verified
10. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Same initial state + same seed + same input log ⇒ identical checkpoint hashes and identical final state hash (Spec 9.2)
2. Replay does not modify the original state objects
3. Replay uses the same execution path and system-dispatch surface as authoritative runtime
4. No nondeterministic operations in replay path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/replay_execution.rs` (inline `#[cfg(test)]`) — determinism proof, error detection, checkpoint verification, recording helper behavior

### Commands

1. `cargo test -p worldwake-sim replay_execution`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
