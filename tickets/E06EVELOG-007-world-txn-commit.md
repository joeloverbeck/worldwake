# E06EVELOG-007: WorldTxn Commit → EventRecord Emission

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds `commit()` to `WorldTxn`, connects to `EventLog::emit`
**Deps**: E06EVELOG-006 (WorldTxn with journaled mutations exists), E06EVELOG-005 (EventLog with all indices exists)

## Problem

`WorldTxn` accumulates deltas but does not yet finalize them into an `EventRecord` or append to the `EventLog`. The commit step is where the journal becomes an immutable causal record — this is the core of the "every persistent mutation has exactly one cause" guarantee.

## Assumption Reassessment (2026-03-09)

1. `WorldTxn` exists from E06EVELOG-006 with `deltas: Vec<StateDelta>`, metadata fields, and `&mut EventLog` — prerequisite
2. `PendingEvent` exists from E06EVELOG-004 as the pre-append event payload without an assigned `EventId` — prerequisite
3. `EventLog::emit(pending_event) -> EventId` exists from E06EVELOG-004 and assigns gapless IDs internally — prerequisite
4. `EventRecord` exists from E06EVELOG-003 with all required fields — prerequisite
5. Secondary indices (by_actor, by_place, by_tag) are maintained by `emit` from E06EVELOG-005 — prerequisite

## Architecture Check

1. `commit()` consumes `WorldTxn` (takes `self`, not `&mut self`) to enforce single-use semantics
2. `commit()` assembles a `PendingEvent` from the accumulated metadata and deltas, then calls `EventLog::emit`
3. `target_ids` are sorted before insertion into the record (spec: "stored in stable sorted order where ordering is not semantically meaningful")
4. An empty `deltas` vec is allowed (e.g. system tick heartbeat events)
5. After commit, the `WorldTxn` is consumed — no further mutations possible

## What to Change

### 1. Add `commit(self) -> EventId` to `WorldTxn`

```rust
pub fn commit(mut self) -> EventId {
    self.target_ids.sort();
    self.target_ids.dedup();

    let pending = PendingEvent {
        tick: self.tick,
        cause: self.cause,
        actor_id: self.actor_id,
        target_ids: self.target_ids,
        place_id: self.place_id,
        state_deltas: self.deltas,
        visibility: self.visibility,
        witness_data: self.witness_data,
        tags: self.tags,
    };

    self.committed = true;
    self.event_log.emit(pending)
}
```

### 2. Add `abort(self)` to `WorldTxn`

Explicit abort that drops without committing. Note: world mutations are already applied (WorldTxn is observational, not transactional), so abort means "don't create an event record for these mutations." This should only be used in error recovery paths.

### 3. Implement `Drop` warning

If `WorldTxn` is dropped without `commit()` or `abort()`, log a debug warning. This helps catch forgotten commits during development.

## Files to Touch

- `crates/worldwake-sim/src/world_txn.rs` (modify — add commit, abort, Drop impl)

## Out of Scope

- Rollback of world mutations on abort (WorldTxn is observational — mutations are immediate)
- Nested transactions
- Multi-event transactions (one WorldTxn = one EventRecord, always)
- Completeness verification (E06EVELOG-009)
- Cause chain traversal (E06EVELOG-008)

## Acceptance Criteria

### Tests That Must Pass

1. `commit()` produces an `EventRecord` in the `EventLog` with the correct `event_id`
2. `commit()` returns the assigned `EventId`
3. After commit, `EventLog::get(id)` returns the record with matching fields
4. `target_ids` in the committed record are sorted and deduplicated
5. `state_deltas` in the committed record preserve mutation order
6. `tags` in the committed record match accumulated tags
7. `commit()` with empty deltas succeeds (system tick events)
8. After commit, `WorldTxn` is consumed — cannot mutate further (compile-time guarantee)
9. Sequential commits from multiple WorldTxn instances produce sequential EventIds
10. EventLog secondary indices are updated correctly after commit
11. `abort()` does not add an event to the log
12. Existing suite: `cargo test --workspace`

### Invariants

1. One WorldTxn commit produces exactly one EventRecord (spec 9.3)
2. Event IDs remain monotonic and gapless after commits
3. Committed deltas are immutable in the log (append-only)
4. `target_ids` are in stable sorted order in the record

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/world_txn.rs` — commit produces correct record, sequential IDs across multiple commits, target sorting, empty deltas allowed, abort leaves log unchanged, end-to-end: create entity → commit → verify in log

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`
