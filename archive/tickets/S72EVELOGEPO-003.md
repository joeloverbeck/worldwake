# S72EVELOGEPO-003: Verification adaptation and integration tests

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — test-only verification infrastructure change
**Deps**: S72EVELOGEPO-001, S72EVELOGEPO-002

## Problem

After compaction runs, events before the checkpoint tick have empty `state_deltas`. The test-only `ExpectedWorldState::from_event_log` in `verification.rs` replays ALL state_deltas to reconstruct world state — it will produce incomplete results for compacted logs because the old deltas are gone. This ticket adapts the verification path to start from the latest checkpoint and replay only the remaining deltas.

Additionally, this ticket adds the integration-level tests that verify compaction preserves determinism and does not affect simulation outcomes: golden test pass, `hash_world()` stability, and soak memory bounds.

## Assumption Reassessment (2026-04-08)

1. `ExpectedWorldState::from_event_log` at `crates/worldwake-core/src/verification.rs:174` is `#[cfg(test)]`. It iterates `0..event_log.len()` and replays all `state_deltas`. It reconstructs `entity_states`, `components`, `relations`, and `reservations` — all of which are present in the serialized `World` checkpoint.
2. `ExpectedWorldState` at `verification.rs:164-170` has fields: `entity_states: BTreeMap<EntityId, EntityKind>`, `components: BTreeMap<(EntityId, ComponentKind), ComponentValue>`, `relations: BTreeSet<RelationValue>`, `reservations: BTreeMap<ReservationId, ReservationRecord>`. To initialize from a deserialized `World`, we need to extract these fields from `World`'s private internals. This requires either: (a) adding `#[cfg(test)]` accessor methods on `World` that expose entity states, components, relations, and reservations as iterators; or (b) using existing public query methods to reconstruct the maps.
3. `EventLog::latest_checkpoint()` from ticket 001 returns `Option<(&Tick, &CheckpointData)>`. `CheckpointData::world_snapshot()` returns `&[u8]` for bincode deserialization.
4. `hash_world()` at `canonical.rs:59` hashes the `World` struct. Compaction does not modify `World`, so `hash_world()` output is identical with and without compaction. This is the determinism check.
5. Golden tests in `crates/worldwake-ai/tests/golden_*.rs` use `Harness` which runs full tick steps. If compaction is enabled by default (interval=50), golden tests will exercise compaction. The golden test hashes must be unaffected — compaction does not change `World` state.
6. Single-layer test infrastructure ticket. The verification code is `#[cfg(test)]` only.

## Architecture Check

1. The adaptation is minimal: if a checkpoint exists, deserialize the `World` from it, extract the reconstruction base, then replay only deltas from that tick forward. The existing full-replay path remains as a fallback for logs with no checkpoints. This preserves the verification's original contract: the reconstructed state must match the live `World`.
2. No backward-compatibility shims. The old full-replay path is a codepath within the same function (the `else` branch), not a separate compatibility layer.

## Verification Layers

1. Checkpoint-based reconstruction produces same result as full replay -> focused unit test (run both paths on same log before compaction, compare)
2. Compacted log reconstruction matches live World state -> focused unit test (compact a log, reconstruct from checkpoint, compare to live World)
3. Golden tests pass with compaction enabled -> golden E2E suite
4. `hash_world()` identical with and without compaction -> focused unit test
5. Single-layer ticket (test infrastructure); no cross-system mapping needed beyond the golden suite.

## What to Change

### 1. Add test-only accessors on `World` for reconstruction

In `crates/worldwake-core/src/world.rs`, add `#[cfg(test)]` methods that expose the data `ExpectedWorldState` needs:

```rust
#[cfg(test)]
impl World {
    pub fn all_entity_kinds(&self) -> impl Iterator<Item = (EntityId, EntityKind)> + '_
    pub fn all_components(&self) -> impl Iterator<Item = ((EntityId, ComponentKind), &ComponentValue)> + '_
    pub fn all_relations(&self) -> impl Iterator<Item = &RelationValue> + '_
    pub fn all_reservations(&self) -> impl Iterator<Item = (ReservationId, &ReservationRecord)> + '_
}
```

The exact signatures depend on the internal data structures. If `World` already exposes sufficient query methods for these, use those instead of adding new ones.

### 2. Adapt `ExpectedWorldState::from_event_log`

In `crates/worldwake-core/src/verification.rs`, modify the function:

```rust
fn from_event_log(event_log: &EventLog) -> Self {
    let (start_index, mut entity_kinds, mut components, mut relations, mut reservations) =
        if let Some((checkpoint_tick, data)) = event_log.latest_checkpoint() {
            let world: World = bincode::deserialize(data.world_snapshot())
                .expect("checkpoint must deserialize");
            // Extract base state from World
            let entity_kinds = /* from world.all_entity_kinds() */;
            let components = /* from world.all_components() */;
            let relations = /* from world.all_relations() */;
            let reservations = /* from world.all_reservations() */;
            // Find first event at or after checkpoint_tick
            let start = /* index of first event at checkpoint_tick */;
            (start, entity_kinds, components, relations, reservations)
        } else {
            (0, BTreeMap::new(), BTreeMap::new(), BTreeSet::new(), BTreeMap::new())
        };

    // Replay deltas from start_index forward (same loop as before)
    for index in start_index..event_log.len() {
        // ... existing delta application logic ...
    }

    Self { entity_states: entity_kinds, components, relations, reservations }
}
```

### 3. Add verification roundtrip test

A new test that:
1. Creates a `World` with entities, components, relations
2. Emits events across multiple ticks
3. Before compaction: calls `from_event_log` and records the result
4. Runs compaction (add checkpoint, strip deltas)
5. After compaction: calls `from_event_log` again
6. Asserts both results are identical

### 4. Add `hash_world` determinism test

A test that:
1. Runs a short simulation (e.g., 100 ticks) with compaction_interval=50
2. Records `hash_world()` at each checkpoint tick
3. Runs the same simulation with compaction_interval=0
4. Asserts `hash_world()` is identical at every tick

### 5. Verify golden tests pass

Run the full golden test suite. Compaction is enabled by default (interval=50 via serde default on ScenarioDef). If golden tests use `ScenarioDef` deserialization, they will pick up the default. If golden tests construct `EventLog` directly via `EventLog::new()`, the interval defaults to 0 (disabled), which is also correct — golden tests continue to work either way.

## Files to Touch

- `crates/worldwake-core/src/verification.rs` (modify — adapt `from_event_log` for checkpoint-based reconstruction)
- `crates/worldwake-core/src/world.rs` (modify — add `#[cfg(test)]` accessors for reconstruction)

## Out of Scope

- Soak binary memory measurements (manual verification, not an automated test)
- Disk-backed storage
- Performance optimization of the compaction function itself
- Any changes to production (non-test) code paths

## Acceptance Criteria

### Tests That Must Pass

1. Checkpoint-based reconstruction produces identical result to full-replay reconstruction
2. Compacted-log reconstruction matches live World state
3. `hash_world()` is identical with compaction enabled and disabled for the same seed
4. All golden tests pass: `cargo test -p worldwake-ai`
5. Existing suite: `cargo test --workspace`

### Invariants

1. `ExpectedWorldState::from_event_log` produces the same result regardless of whether the log has been compacted, as long as the checkpoint covers all stripped events
2. `hash_world()` is independent of `EventLog` compaction state
3. No production code paths are modified — all changes are `#[cfg(test)]`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/verification.rs` — verification_roundtrip_with_compaction (both paths produce same result)
2. `crates/worldwake-core/src/verification.rs` — compacted_log_matches_live_world
3. `crates/worldwake-core/src/canonical.rs` or `verification.rs` — hash_world_stable_across_compaction

### Commands

1. `cargo test -p worldwake-core` (targeted — verification and canonical tests)
2. `cargo test -p worldwake-ai` (golden tests)
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Completed on 2026-04-08.

- Adapted `ExpectedWorldState::from_event_log` in `verification.rs` to use checkpoint-based reconstruction: if `latest_checkpoint()` exists, deserializes `World` from it, extracts base state via `ActualWorldState::from_world`, then replays only remaining deltas
- Added 3 new tests: `verification_roundtrip_with_compaction`, `compacted_log_verification_matches_live_world`, `hash_world_stable_across_compaction`
- No changes to `world.rs` — reused existing `ActualWorldState::from_world` instead of adding new test-only accessors

Deviation from ticket: Ticket proposed adding `#[cfg(test)]` accessor methods on `World` (`all_entity_kinds`, `all_components`, `all_relations`, `all_reservations`). Not needed — `ActualWorldState::from_world` already provides this extraction via public `World` query methods, making the adaptation simpler with zero production code changes.

## Verification Result

- Passed `cargo test -p worldwake-core -- verification_roundtrip_with_compaction compacted_log_verification hash_world_stable` (3 focused tests)
- Passed `cargo clippy --workspace --all-targets -- -D warnings` (clean)
- Passed `cargo test --workspace` (all suites, 0 failures, 1045 core tests)
