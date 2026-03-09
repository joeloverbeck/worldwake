# E08TIMSCHREP-008: ReplayState — recording infrastructure

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: E08TIMSCHREP-003 (InputEvent), E08TIMSCHREP-007 (StateHash, canonical hashing)
**Spec Ref**: `specs/E08-time-scheduler-replay.corrected.md` ("Replay Recording", "Replay Execution")

## Problem

Replay verification requires recording: the initial state hash, master seed, ordered input log, and per-tick checkpoint hashes. Without a dedicated `ReplayState` struct, this metadata would need to be reconstructed from scattered sources, making replay fragile and hard to test.

## Assumption Reassessment (2026-03-09)

1. `StateHash` already exists in `worldwake-core` via the completed canonical hashing work from E08TIMSCHREP-007 — confirmed.
2. `InputEvent` already exists in `worldwake-sim` and already carries deterministic `(scheduled_tick, sequence_no)` ordering — confirmed.
3. `Scheduler` already owns the future-facing `InputQueue`, but that queue is drained as ticks execute. Replay recording therefore still needs its own append-only historical input log instead of aliasing scheduler state — confirmed.
4. `step_tick` already provides the deterministic execution boundary and emits an end-of-tick marker, but it does not yet record replay metadata — confirmed.
5. `Seed` and `Tick` already exist in `worldwake-core` and fit replay metadata directly — confirmed.
6. No `ReplayState`, replay checkpoint type, or replay recording config exists yet — confirmed.

## Architecture Check

1. `ReplayState` must stay a passive, serializable recording object. It records replay facts; it does not own scheduling, execution, or verification logic.
2. `ReplayState` should sit beside `Scheduler`, not inside it. The scheduler owns mutable execution state; replay recording owns immutable historical facts accumulated over time.
3. Checkpoint cadence should not use a `0 = never` sentinel. A disabled/enabled configuration is cleaner and less error-prone than encoding behavior in a magic number.
4. Checkpoint ordering should be enforced by the API, not left as a comment-level invariant. Recording a checkpoint with a duplicate or earlier tick should return an error.
5. Separation remains: recording infrastructure belongs here; tick-loop wiring and replay verification belong in later E08 work.

## Scope Correction

This ticket should provide the replay recording data model only:

1. Store the initial state hash, master seed, append-only input log, append-only checkpoint log, and checkpoint policy.
2. Enforce replay-recording invariants locally, especially monotonic checkpoint ticks.
3. Avoid coupling to `Scheduler`, `InputQueue`, or `step_tick` internals beyond using shared types.

This ticket should not:

1. Introduce tick-loop wiring into `step_tick`.
2. Reconstruct or mirror the scheduler's pending input queue.
3. Perform replay execution, checkpoint comparison, or determinism verification.

## What to Change

### 1. New type: `ReplayCheckpoint`

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplayCheckpoint {
    pub tick: Tick,
    pub event_log_hash: StateHash,
    pub world_state_hash: StateHash,
}
```

### 2. New type: `ReplayRecordingConfig`

```rust
use std::num::NonZeroU64;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplayRecordingConfig {
    pub checkpoint_interval: Option<NonZeroU64>,
}
```

Required constructors/accessors:

- `disabled() -> Self`
- `every(interval: NonZeroU64) -> Self`
- `checkpoint_interval(&self) -> Option<NonZeroU64>`

### 3. New type: `ReplayState`

```rust
pub struct ReplayState {
    initial_state_hash: StateHash,
    master_seed: Seed,
    input_log: Vec<InputEvent>,
    checkpoints: Vec<ReplayCheckpoint>,
    config: ReplayRecordingConfig,
}
```

Methods:
- `new(initial_hash: StateHash, seed: Seed, config: ReplayRecordingConfig) -> Self`
- `record_input(&mut self, input: InputEvent)` — appends to ordered input log
- `record_checkpoint(&mut self, checkpoint: ReplayCheckpoint) -> Result<(), ReplayStateError>` — appends checkpoint if tick order is strictly increasing
- `should_checkpoint(&self, tick: Tick) -> bool` — checks against interval config
- `initial_state_hash(&self) -> StateHash`
- `master_seed(&self) -> Seed`
- `config(&self) -> &ReplayRecordingConfig`
- `input_log(&self) -> &[InputEvent]`
- `checkpoints(&self) -> &[ReplayCheckpoint]`

### 4. New type: `ReplayStateError`

```rust
pub enum ReplayStateError {
    NonMonotonicCheckpoint {
        previous_tick: Tick,
        attempted_tick: Tick,
    },
}
```

Derives:

- `ReplayCheckpoint`: `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`
- `ReplayRecordingConfig`: `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`
- `ReplayState`: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`
- `ReplayStateError`: `Clone, Debug, Eq, PartialEq`

## Files to Touch

- `crates/worldwake-sim/src/replay_state.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Replay execution/verification (E08TIMSCHREP-009)
- Integrating recording into `step_tick` (E08TIMSCHREP-009 or a follow-up wiring ticket)
- Save/load of replay state (handled by SimulationState serialization in E08TIMSCHREP-011)
- Streaming replay to disk

## Acceptance Criteria

### Tests That Must Pass

1. `ReplayState` satisfies `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
2. `ReplayCheckpoint` satisfies `Copy + Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
3. `ReplayRecordingConfig` satisfies `Copy + Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
4. `ReplayState::new` stores initial hash, seed, and config correctly
5. `record_input` preserves insertion order
6. `record_checkpoint` preserves insertion order for strictly increasing ticks
7. `record_checkpoint` rejects duplicate or earlier ticks with `ReplayStateError::NonMonotonicCheckpoint`
8. `should_checkpoint(tick)` returns `true` at correct intervals (for example, every 5 ticks => true at ticks 0, 5, 10)
9. `ReplayRecordingConfig::disabled()` causes `should_checkpoint` to always return `false`
10. `ReplayRecordingConfig::every(NonZeroU64::new(1).unwrap())` causes `should_checkpoint` to always return `true`
11. Bincode round-trip for `ReplayState` with populated inputs and checkpoints
12. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Input log order matches recording order (append-only)
2. Checkpoint order matches tick order (append-only, strictly increasing tick)
3. No `HashMap`/`HashSet` in replay state
4. Replay recording remains decoupled from scheduler execution state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/replay_state.rs` (inline `#[cfg(test)]`) — construction, recording, checkpointing, monotonicity enforcement, serialization

### Commands

1. `cargo test -p worldwake-sim replay_state`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

Completion date: 2026-03-09

What actually changed:

1. Added `crates/worldwake-sim/src/replay_state.rs` with `ReplayCheckpoint`, `ReplayRecordingConfig`, `ReplayState`, and `ReplayStateError`.
2. Implemented append-only input recording plus strictly increasing checkpoint recording with explicit rejection of duplicate or earlier checkpoint ticks.
3. Used `Option<NonZeroU64>` for checkpoint cadence instead of the original `0 = never` sentinel design.
4. Re-exported the replay recording types from `crates/worldwake-sim/src/lib.rs`.

Deviations from original plan:

1. The ticket was corrected before implementation to match the current architecture: replay recording remains passive and separate from `Scheduler` and `InputQueue`.
2. The checkpoint-order invariant is enforced by `record_checkpoint(...) -> Result<..., ReplayStateError>` rather than documented only as an invariant.
3. `ReplayCheckpoint` and `ReplayRecordingConfig` were implemented as `Copy` because their fields are value types and the stronger trait set is useful for deterministic replay code.

Verification results:

1. `cargo test -p worldwake-sim replay_state` passed.
2. `cargo test -p worldwake-sim` passed.
3. `cargo clippy --workspace` passed.
4. `cargo test --workspace` passed.
