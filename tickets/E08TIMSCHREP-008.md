# E08TIMSCHREP-008: ReplayState — recording infrastructure

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: E08TIMSCHREP-003 (InputEvent), E08TIMSCHREP-007 (StateHash, canonical hashing)

## Problem

Replay verification requires recording: the initial state hash, master seed, ordered input log, and per-tick checkpoint hashes. Without a dedicated `ReplayState` struct, this metadata would need to be reconstructed from scattered sources, making replay fragile and hard to test.

## Assumption Reassessment (2026-03-09)

1. `StateHash` from E08TIMSCHREP-007 provides canonical hashing
2. `InputEvent` from E08TIMSCHREP-003 provides the ordered input type
3. `Seed` from worldwake-core provides the master seed type
4. `Tick` from worldwake-core provides the tick type
5. No replay state exists yet — confirmed

## Architecture Check

1. `ReplayState` is a passive data structure — it records what happened, it does not drive replay
2. Checkpoint intervals are configurable (e.g., every N ticks) — stored as a setting in `ReplayState`
3. The replay recorder is invoked by the tick step (E08TIMSCHREP-006), but recording logic lives here
4. Separation: recording (this ticket) vs execution/verification (E08TIMSCHREP-009)

## What to Change

### 1. New type: `ReplayCheckpoint`

```rust
pub struct ReplayCheckpoint {
    pub tick: Tick,
    pub event_log_hash: StateHash,
    pub world_state_hash: StateHash,
}
```

### 2. New type: `ReplayRecordingConfig`

```rust
pub struct ReplayRecordingConfig {
    pub checkpoint_interval: u64,  // record checkpoint every N ticks; 0 = never, 1 = every tick
}
```

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
- `record_checkpoint(&mut self, checkpoint: ReplayCheckpoint)` — appends checkpoint
- `should_checkpoint(&self, tick: Tick) -> bool` — checks against interval config
- `initial_state_hash(&self) -> StateHash`
- `master_seed(&self) -> Seed`
- `input_log(&self) -> &[InputEvent]`
- `checkpoints(&self) -> &[ReplayCheckpoint]`

All types derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

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
2. `ReplayCheckpoint` satisfies same trait set
3. `ReplayRecordingConfig` satisfies same trait set
4. `ReplayState::new` stores initial hash and seed correctly
5. `record_input` preserves insertion order
6. `record_checkpoint` preserves insertion order
7. `should_checkpoint(tick)` returns `true` at correct intervals (e.g., interval=5: true at tick 0, 5, 10)
8. `should_checkpoint` with interval=0 always returns `false`
9. `should_checkpoint` with interval=1 always returns `true`
10. Bincode round-trip for `ReplayState` with populated inputs and checkpoints
11. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Input log order matches recording order (append-only)
2. Checkpoint order matches tick order (append-only, monotonically increasing tick)
3. No `HashMap`/`HashSet` in replay state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/replay_state.rs` (inline `#[cfg(test)]`) — construction, recording, checkpointing, serialization

### Commands

1. `cargo test -p worldwake-sim replay_state`
2. `cargo clippy --workspace && cargo test --workspace`
