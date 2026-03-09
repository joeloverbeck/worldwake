# E08TIMSCHREP-009: Replay execution and verification

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: E08TIMSCHREP-006 (step_tick), E08TIMSCHREP-007 (canonical hashing), E08TIMSCHREP-008 (ReplayState)
**Spec Ref**: `specs/E08-time-scheduler-replay.corrected.md` ("Replay Recording", "Replay Execution", "Per-Tick Flow")

## Problem

The replay system must prove determinism: given the same initial state, same seed, and same input log, the simulation must produce identical checkpoint hashes and identical final state. This is the T08 gate test. Without a replay executor, determinism is a claim, not a proven property.

## Assumption Reassessment (2026-03-09)

1. `step_tick` already defines the authoritative tick boundary and already consumes a `TickStepServices` bundle containing action registries plus `SystemDispatchTable` — confirmed.
2. `ReplayState` now exists and stores the initial state hash, master seed, terminal replay tick, ordered input log, replay checkpoints, and checkpoint policy — confirmed.
3. `ReplayCheckpoint` is already a concrete pair of hashes per tick (`event_log_hash`, `world_state_hash`). The current architecture does not use a kind-tagged checkpoint stream, so replay verification should compare those two fields directly rather than introducing a second checkpoint representation — confirmed.
4. `StateHash`, `hash_world`, `hash_event_log`, and generic `hash_serializable` already exist in `worldwake-core`. The initial/final replay proof can therefore hash the explicit authoritative roots directly without waiting for `SimulationState` — confirmed.
5. `DeterministicRng` reconstructs from `Seed` and uses deterministic substreams per `(tick, system_id, seq)` — confirmed.
6. `Scheduler` owns the mutable `InputQueue`; replay should rebuild that queue from the recorded `ReplayState::input_log()` on a cloned scheduler, not invent a parallel execution path — confirmed.
7. `ReplayState` now functions as the authoritative ordered replay input log, and replay infrastructure can seed that log from an initial scheduler queue and rebuild queued inputs exactly, including nonzero sequence offsets.
8. Replay length must be explicit. `ReplayState` now carries a terminal tick so exact replay duration does not depend on the existence of later inputs or checkpoints.
9. There is no `SystemRegistry` type in the current architecture. Runtime systems are supplied through `SystemDispatchTable`, and replay must use that same interface to stay aligned with normal execution — confirmed.
10. There is no `SimulationState` root type yet. Replay execution should therefore operate on explicit state roots (`World`, `EventLog`, `Scheduler`, `ControllerState`) until E08TIMSCHREP-010 introduces a consolidated simulation root.

## Architecture Check

1. Replay must reuse the exact production tick path (`step_tick`) rather than a replay-specific simulation loop. Determinism proof is strongest when replay exercises the same scheduler, action, and system boundaries as live execution.
2. Replay should clone the provided authoritative roots and seed a fresh `DeterministicRng` from the recording's master seed. The original state objects must remain untouched.
3. The replay API should accept the same registries/dispatch objects that `step_tick` already needs. Introducing a separate replay-specific registry abstraction would be architectural duplication.
4. Checkpoint verification should compare recorded `ReplayCheckpoint` entries directly, in recorded order, and should report mismatches with concrete tick plus concrete field context (`event_log_hash` vs `world_state_hash`). A `CheckpointKind` abstraction would add indirection without buying extensibility in the current checkpoint model.
5. Initial and final replay verification should hash the explicit authoritative roots (`World`, `EventLog`, `Scheduler`, `ControllerState`) with `hash_serializable(...)`. This is stricter than hashing world-only state and better matches the architecture's notion of authoritative state before `SimulationState` exists.
6. Recording helpers that compute and append checkpoints after a tick step are appropriate here because they are shared replay infrastructure and keep hashing/append rules out of `step_tick` itself.
7. Replay should replace the cloned scheduler input queue from the authoritative replay input log rather than regenerating inputs with fresh sequence numbers or relying on the caller's initial queue shape.

## Scope Correction

This ticket should:

1. Implement replay execution and verification on top of the existing `step_tick` + `TickStepServices` architecture.
2. Add a shared helper for recording per-tick checkpoints into `ReplayState`.
3. Define replay verification in terms of the existing `ReplayCheckpoint` shape and explicit authoritative-root hashing.
4. Record an explicit terminal replay tick so replay duration is exact even for idle runs with no later inputs or checkpoints.
5. Provide the T08 determinism proof tests for the current Phase 1 scheduler stack.

This ticket should not:

1. Introduce a shadow replay runner with different execution phases.
2. Depend on a not-yet-existing `SimulationState` root.
3. Rework scheduler ownership boundaries established in E08TIMSCHREP-006 and E08TIMSCHREP-008.
4. Introduce a shadow replay duration heuristic outside `ReplayState`.

## What to Change

### 1. New type: `ReplayError`

```rust
pub enum ReplayError {
    InitialStateHashMismatch { expected: StateHash, actual: StateHash },
    EventLogCheckpointMismatch {
        tick: Tick,
        expected: StateHash,
        actual: StateHash,
    },
    WorldCheckpointMismatch {
        tick: Tick,
        expected: StateHash,
        actual: StateHash,
    },
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
2. Verify the canonical hash of the explicit authoritative roots `(World, EventLog, Scheduler, ControllerState)` matches `replay.initial_state_hash()`
3. Clone `World`, `EventLog`, `Scheduler`, and `ControllerState`
4. Replace the cloned scheduler's `InputQueue` from `replay.input_log()` while preserving exact recorded `InputEvent` sequence numbers
5. Step through ticks with `step_tick(...)`, reusing the provided `TickStepServices`
6. After each stepped tick, compare any expected checkpoint for that tick against freshly computed `hash_event_log(...)` and `hash_world(...)`
7. Replay until `replay.terminal_tick()` is reached, then compute and return the canonical final-state hash of `(World, EventLog, Scheduler, ControllerState)`
8. Return the final hash on success, or accumulated replay errors from initial/checkpoint verification

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

Additional shared helper:

```rust
pub fn seed_replay_inputs_from_scheduler(
    replay: &mut ReplayState,
    scheduler: &Scheduler,
) -> Result<usize, ReplayCheckpointError>
```

Behavior:

1. Iterate the scheduler's queued inputs in global `sequence_no` order.
2. Append them into `ReplayState::input_log()` using replay-state ordering validation.
3. Return the number of seeded pending inputs.

Replay recording also updates `ReplayState::terminal_tick` as ticks complete so idle replay duration remains explicit.

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
4. Replay detects initial state hash mismatch when given wrong initial world/event-log/scheduler/controller roots
5. Replay reconstructs an initial non-empty scheduler input queue and nonzero input sequence offset faithfully from `ReplayState::input_log()`
6. Replay detects checkpoint mismatch when state is corrupted mid-replay (inject a mutation)
7. Replay with no later inputs and no checkpoints still replays the exact recorded number of idle ticks via `ReplayState::terminal_tick`
8. `ReplayError` variants are descriptive (tick, expected/actual hashes)
9. `record_tick_checkpoint` records nothing when `ReplayRecordingConfig::disabled()` is used
10. Replay with `ReplayRecordingConfig::every(NonZeroU64::new(1).unwrap())`: every tick verified
11. Replay with `ReplayRecordingConfig::every(NonZeroU64::new(5).unwrap())`: only ticks 0, 5, 10, ... verified
12. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Same initial authoritative roots + same seed + same input log ⇒ identical checkpoint hashes and identical final state hash (Spec 9.2)
2. Replay does not modify the original state objects
3. Replay uses the same execution path and system-dispatch surface as authoritative runtime
4. No nondeterministic operations in replay path
5. Replay reconstructs queued input state from authoritative replay data instead of approximating it with regenerated sequence numbers

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/replay_execution.rs` (inline `#[cfg(test)]`) — determinism proof, error detection, checkpoint verification, recording helper behavior

### Commands

1. `cargo test -p worldwake-sim replay_execution`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

Completion date: 2026-03-09

What actually changed:

1. Added `crates/worldwake-sim/src/replay_execution.rs` with `ReplayError`, `ReplayCheckpointError`, `record_tick_checkpoint(...)`, and `replay_and_verify(...)`.
2. Replay verification now hashes the explicit authoritative roots `(World, EventLog, Scheduler, ControllerState)` for initial/final comparison instead of treating world-only hashing as sufficient before `SimulationState` exists.
3. Replay reuses `step_tick(...)` directly and verifies recorded `ReplayCheckpoint` entries against freshly computed `event_log_hash` and `world_state_hash` values.
4. Added `seed_replay_inputs_from_scheduler(...)` plus `InputQueue` replay helpers so recording can capture an initial scheduler queue and replay can rebuild it exactly, including nonzero sequence offsets.
5. Added an explicit `ReplayState::terminal_tick` boundary so replay duration is exact even when a recording ends on idle ticks with no later inputs or checkpoints.
6. Added deterministic `InputQueue` iteration/rebuild APIs plus replay-state input-order validation to make the replay log a stricter authoritative record.

Differences from the original plan:

1. The original `CheckpointKind` abstraction and single `CheckpointMismatch` variant were dropped. The implemented architecture matches the concrete `ReplayCheckpoint` shape already present in the codebase with separate world/event-log mismatch variants.
2. The original `FinalStateHashMismatch` variant was dropped because `ReplayState` does not record an expected final hash. The replay API now returns the computed final composite hash for callers/tests to compare.
3. The intermediate rejection-based queue limitation was removed. Replay now captures and reconstructs initial queued inputs explicitly via authoritative replay log data.
4. Replay termination is now explicit in recording metadata rather than inferred from the last input/checkpoint boundary.

Verification results:

1. `cargo test -p worldwake-sim replay_execution` passed.
2. `cargo test -p worldwake-sim` passed.
3. `cargo clippy --workspace --all-targets -- -D warnings` passed.
4. `cargo test --workspace` passed.
