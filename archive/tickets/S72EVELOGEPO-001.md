# S72EVELOGEPO-001: Checkpoint storage and delta stripping infrastructure

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — EventLog gains checkpoint storage and compaction_interval; EventRecord gains strip_state_deltas
**Deps**: S71 (event log delta compaction) — COMPLETED

## Problem

The append-only event log grows unboundedly in RAM. Even after S71 reduced per-event delta size to ~1-5 KB, the cumulative `state_deltas` payload across all events reaches hundreds of MB within thousands of ticks. The simulation must run indefinitely as a game backbone, so RAM must be bounded.

This ticket adds the core infrastructure: checkpoint storage on `EventLog`, a compaction interval field, methods to create/query/prune checkpoints and strip old deltas, and `strip_state_deltas` on `EventRecord`. No callers are wired yet — that is ticket 002.

## Assumption Reassessment (2026-04-08)

1. `EventLog` at `crates/worldwake-core/src/event_log.rs:9` derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Fields are all private. Adding `checkpoints: BTreeMap<Tick, CheckpointData>` and `compaction_interval: u32` is additive and does not break existing API. `CheckpointData` must also derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` to satisfy `EventLog`'s derives.
2. `EventRecord` at `crates/worldwake-core/src/event_record.rs:62` has private `payload: EventPayload`. `EventPayload` at line 46 has `pub state_deltas: Vec<StateDelta>`. Adding `strip_state_deltas(&mut self)` on both types requires `&mut self` access — `EventRecord`'s `payload` is private but the method lives on `EventRecord` itself, so it can access `payload` internally.
3. `World` at `crates/worldwake-core/src/world.rs:120` derives `Serialize, Deserialize`. Bincode serialization of `World` is the checkpoint format. `World` contains `EntityAllocator`, `ComponentTables`, `RelationTables`, `Topology` — all needed for reconstruction per FND-12.
4. `EventLog::new()` at `event_log.rs:21` must be updated to initialize `checkpoints: BTreeMap::new()` and `compaction_interval: 0` (disabled by default).
5. `hash_event_log()` at `canonical.rs:63` serializes the entire `EventLog` via bincode. Adding new fields will change the hash. This is expected and documented in S72 — `hash_world()` is the determinism check, not `hash_event_log()`.
6. Single-layer infrastructure ticket in worldwake-core. No cross-system boundary under audit.

## Architecture Check

1. Checkpoint storage on `EventLog` is self-contained: the event log owns its compaction lifecycle. No external system needs to know about checkpoints — they are an internal storage optimization. This follows FND-27 (checkpoints are derived caches) and FND-29A (events are compacted for storage, not deleted).
2. No backward-compatibility shims. The `compaction_interval` defaults to 0 (disabled), so existing code that constructs `EventLog::new()` continues to work without checkpoints.

## Verification Layers

1. Checkpoint roundtrip (serialize World, deserialize, compare) -> focused unit test
2. Strip correctness (events before cutoff have empty deltas, after cutoff are unchanged) -> focused unit test
3. Checkpoint pruning (max 2 retained) -> focused unit test
4. `EventLog` serialization stability (new fields serialize/deserialize correctly) -> focused unit test
5. Single-layer ticket in worldwake-core; no cross-system or mixed-layer mapping needed.

## What to Change

### 1. Define `CheckpointData` struct

In `crates/worldwake-core/src/event_log.rs`, add:

```rust
/// Serialized World snapshot at a specific tick.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CheckpointData {
    /// Bincode-serialized World state.
    world_snapshot: Vec<u8>,
}
```

### 2. Add checkpoint fields to `EventLog`

Add two fields to the `EventLog` struct:

```rust
/// Periodic World snapshots. Key: tick.
/// At most 2 retained (current + previous for safety).
checkpoints: BTreeMap<Tick, CheckpointData>,

/// Ticks between checkpoint snapshots. 0 = disabled.
compaction_interval: u32,
```

Update `EventLog::new()` to initialize both fields with defaults (`BTreeMap::new()`, `0`).

### 3. Add checkpoint methods on `EventLog`

```rust
pub fn set_compaction_interval(&mut self, interval: u32)
pub fn compaction_interval(&self) -> u32
pub fn add_checkpoint(&mut self, tick: Tick, data: CheckpointData)
pub fn latest_checkpoint(&self) -> Option<(&Tick, &CheckpointData)>
pub fn prune_old_checkpoints(&mut self, max: usize)
pub fn strip_deltas_before(&mut self, cutoff_tick: Tick)
```

`strip_deltas_before` iterates events using the `by_tick` index for ticks < `cutoff_tick` and calls `strip_state_deltas()` on each matching `EventRecord`.

`add_checkpoint` inserts into the `checkpoints` BTreeMap.

`latest_checkpoint` returns `self.checkpoints.iter().next_back()` (last entry in BTreeMap = most recent tick).

`prune_old_checkpoints(max)` removes oldest entries until `checkpoints.len() <= max`.

### 4. Add `strip_state_deltas` on `EventRecord` and `EventPayload`

In `crates/worldwake-core/src/event_record.rs`:

```rust
impl EventRecord {
    pub(crate) fn strip_state_deltas(&mut self) {
        self.payload.state_deltas.clear();
        self.payload.state_deltas.shrink_to_fit();
    }
}
```

The method is `pub(crate)` because only `EventLog::strip_deltas_before` calls it, and both types are in worldwake-core.

Note: `EventLog::events` is `Vec<EventRecord>` (not `Vec<&EventRecord>`), so `strip_deltas_before` can iterate `&mut self.events[index]` to get mutable references. The `by_tick` index maps `Tick -> Vec<EventId>` where `EventId(n)` is the index into `self.events`.

### 5. Expose `CheckpointData.world_snapshot` for deserialization

Add a getter on `CheckpointData`:

```rust
impl CheckpointData {
    pub fn new(world_snapshot: Vec<u8>) -> Self {
        Self { world_snapshot }
    }

    pub fn world_snapshot(&self) -> &[u8] {
        &self.world_snapshot
    }
}
```

### 6. Unit tests

Add tests in `event_log.rs` (or a `tests` submodule):

- `checkpoint_roundtrip`: create a `World`, serialize via `bincode::serialize`, store as `CheckpointData`, add to `EventLog`, retrieve via `latest_checkpoint`, deserialize, compare to original.
- `strip_deltas_before_correctness`: emit events at ticks 1-10, call `strip_deltas_before(Tick(5))`, assert events at ticks 1-4 have empty `state_deltas`, events at ticks 5-10 are unchanged.
- `checkpoint_pruning`: add 3 checkpoints at ticks 50, 100, 150, call `prune_old_checkpoints(2)`, assert only ticks 100 and 150 remain.
- `compaction_interval_default`: `EventLog::new()` has `compaction_interval() == 0`.
- `set_compaction_interval`: set to 50, verify getter returns 50.

## Files to Touch

- `crates/worldwake-core/src/event_log.rs` (modify — add `CheckpointData`, fields, methods, tests)
- `crates/worldwake-core/src/event_record.rs` (modify — add `strip_state_deltas`)

## Out of Scope

- Wiring compaction_interval from ScenarioDef (ticket 002)
- Compaction SystemFn implementation and registration (ticket 002)
- Verification adaptation (ticket 003)
- Any changes to worldwake-sim or worldwake-cli
- Disk-backed storage
- Delta compaction by merging diffs

## Acceptance Criteria

### Tests That Must Pass

1. `checkpoint_roundtrip` — World serialization/deserialization through CheckpointData is lossless
2. `strip_deltas_before_correctness` — only events before cutoff tick are stripped
3. `checkpoint_pruning` — oldest checkpoints are dropped when exceeding max
4. `compaction_interval_default` — new EventLog has interval 0
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `EventLog::new()` produces a valid log with no checkpoints and compaction disabled
2. `strip_deltas_before` never modifies events at or after the cutoff tick
3. `prune_old_checkpoints(2)` never drops the most recent 2 checkpoints
4. `CheckpointData` satisfies all trait bounds required by `EventLog`'s derives (Clone, Debug, Eq, PartialEq, Serialize, Deserialize)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_log.rs` (tests module) — checkpoint_roundtrip, strip_deltas_before_correctness, checkpoint_pruning, compaction_interval_default, set_compaction_interval

### Commands

1. `cargo test -p worldwake-core` (targeted)
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Completed on 2026-04-08.

- Added `CheckpointData` struct with `new()` and `world_snapshot()` methods to `event_log.rs`
- Added `checkpoints: BTreeMap<Tick, CheckpointData>` and `compaction_interval: u32` fields to `EventLog`
- Added 6 public methods on `EventLog`: `set_compaction_interval`, `compaction_interval`, `add_checkpoint`, `latest_checkpoint`, `prune_old_checkpoints`, `strip_deltas_before`
- Added `strip_state_deltas()` method on `EventRecord` (pub(crate))
- Added `CheckpointData` to crate-root re-exports in `lib.rs`
- Updated `from_records_for_test` to include new fields
- Added 12 new unit tests covering all checkpoint and stripping operations

Auto-correction: `from_records_for_test` at `event_log.rs:177` manually constructs `EventLog` with hardcoded fields — updated to include `checkpoints` and `compaction_interval` fields.

## Verification Result

- Passed `cargo test -p worldwake-core -- checkpoint compaction_interval strip_deltas` (12 focused tests)
- Passed `cargo test -p worldwake-core` (full crate, 1042+ tests)
- Passed `cargo clippy --workspace --all-targets -- -D warnings` (clean)
- Passed `cargo test --workspace` (all suites, 0 failures)
