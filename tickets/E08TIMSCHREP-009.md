# E08TIMSCHREP-009: Replay execution and verification

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: E08TIMSCHREP-006 (step_tick), E08TIMSCHREP-007 (canonical hashing), E08TIMSCHREP-008 (ReplayState)

## Problem

The replay system must prove determinism: given the same initial state, same seed, and same input log, the simulation must produce identical checkpoint hashes and identical final state. This is the T08 gate test. Without a replay executor, determinism is a claim, not a proven property.

## Assumption Reassessment (2026-03-09)

1. `step_tick` from E08TIMSCHREP-006 drives the tick loop
2. `ReplayState` from E08TIMSCHREP-008 holds initial hash, seed, input log, checkpoints
3. `StateHash` and `hash_world`/`hash_event_log` from E08TIMSCHREP-007 provide hashing
4. `DeterministicRng` from E08TIMSCHREP-002 reconstructs from seed
5. `InputQueue` from E08TIMSCHREP-003 can be populated from the recorded input log
6. `Scheduler`, `ControllerState`, `SystemRegistry` from prior tickets provide the tick context

## Architecture Check

1. Replay is a function that takes an initial `SimulationState` snapshot + `ReplayState` recording, then re-executes all ticks, comparing hashes at checkpoints
2. The replay function does NOT modify the original state — it works on a clone
3. Mismatches produce a `ReplayError` with the tick number and expected/actual hashes
4. This is the key integration test infrastructure for Phase 1 gate

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
    systems: &SystemRegistry,
) -> Result<StateHash, Vec<ReplayError>>
```

Flow:
1. Reconstruct `DeterministicRng` from `replay.master_seed()`
2. Verify initial state hash matches `replay.initial_state_hash()`
3. Populate `InputQueue` from `replay.input_log()`
4. Clone all mutable state
5. Step through ticks, calling `step_tick` for each
6. At checkpoint ticks, compute hashes and compare against recorded checkpoints
7. After all inputs are exhausted and final tick reached, compute final state hash
8. Return final hash on success, or accumulated errors

### 3. Integration with recording

Add a helper function `record_tick_checkpoint` that can be called after `step_tick` to record the checkpoint into `ReplayState` if the tick interval matches. This wires E08TIMSCHREP-008's recording into the tick loop.

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
7. Replay with checkpoint_interval=1: every tick verified
8. Replay with checkpoint_interval=5: only ticks 0, 5, 10, ... verified
9. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Same initial state + same seed + same input log ⇒ identical checkpoint hashes and identical final state hash (Spec 9.2)
2. Replay does not modify the original state objects
3. No nondeterministic operations in replay path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/replay_execution.rs` (inline `#[cfg(test)]`) — determinism proof, error detection, checkpoint verification

### Commands

1. `cargo test -p worldwake-sim replay_execution`
2. `cargo clippy --workspace && cargo test --workspace`
