# E06EVELOG-007: WorldTxn Commit → EventRecord Emission

## Archive Amendment (2026-03-09)

The final authoritative `WorldTxn` and `EventLog` implementation lives in `worldwake-core`, not `worldwake-sim`. This archived ticket preserves the intermediate commit-path plan before that consolidation.

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds event emission/commit on top of `WorldTxn`, connecting the existing mutation journal to `EventLog::emit`
**Deps**: E06EVELOG-006 (WorldTxn with journaled mutations exists), E06EVELOG-005 (EventLog with all indices exists)

## Problem

`WorldTxn` already journals canonical state deltas while mutating `World`, but it does not yet finalize that journal into an immutable event in `EventLog`. Until that commit path exists, E06's causal-completeness guarantee remains incomplete because journaled world writes can still happen without becoming durable causal records.

## Assumption Reassessment (2026-03-09)

1. `WorldTxn` exists from E06EVELOG-006 with journal metadata, `target_ids`, `tags`, `visibility`, `witness_data`, and ordered `deltas`, plus immutable read-through over `World` — confirmed
2. `PendingEvent` exists in `crates/worldwake-sim/src/event_record.rs` as the pre-append payload without an assigned `EventId` — confirmed
3. `EventLog::emit(pending_event) -> EventId` exists in `crates/worldwake-sim/src/event_log.rs`, assigns gapless IDs internally, and maintains secondary indices — confirmed
4. `PendingEvent::new(...)` already canonicalizes `target_ids` by sorting and deduplicating them, so `WorldTxn::commit` should delegate that invariant instead of reimplementing it — confirmed
5. `WorldTxn` already journals composite archive teardown along with create, placement, and reservation mutations, so commit must preserve multi-delta batches exactly as accumulated — confirmed
6. `WorldTxn` currently has no lifecycle state beyond ownership; forgetting to commit would silently drop the only causal record for already-applied world mutations — confirmed gap

## Architecture Check

1. `commit()` should consume `WorldTxn` (`self`, not `&mut self`) so one journal can become at most one event
2. `commit()` should assemble a `PendingEvent` from the transaction metadata and accumulated deltas, then delegate record finalization and ID assignment to `EventLog::emit`
3. Because `WorldTxn` does not own `EventLog`, the clean boundary is `commit(self, event_log: &mut EventLog) -> EventId`
4. `target_ids` canonicalization belongs in `PendingEvent::new`, which is already the single event-construction choke point; duplicating that sort/dedup work in `WorldTxn` would weaken the architecture by splitting one invariant across two layers
5. Empty `deltas` must remain valid so root/system events can exist without fake mutations
6. An explicit `abort()` is not desirable here: because `WorldTxn` mutates `World` immediately and has no rollback, `abort()` would become a sanctioned way to persist world changes without an event record, directly undermining E06's causal-completeness goal

## What to Change

### 1. Add `commit(self, event_log: &mut EventLog) -> EventId` to `WorldTxn`

```rust
pub fn commit(self, event_log: &mut EventLog) -> EventId {
    let pending = PendingEvent::new(
        self.tick,
        self.cause,
        self.actor_id,
        self.target_ids,
        self.place_id,
        self.deltas,
        self.visibility,
        self.witness_data,
        self.tags,
    );

    event_log.emit(pending)
}
```

### 2. Do not add `abort(self)` in this ticket

With the current immediate-mutation journal architecture, `abort()` would not undo anything; it would only normalize "mutated world, no event emitted." That is the opposite of the invariant this epic is trying to establish. If a future design needs recoverable staging, that should be a real transactional/rollback design, not an alias for dropping the journal.

### 3. Skip `Drop` warnings for now

A `Drop` warning may still be useful later, but it is secondary to establishing the canonical commit path. It also introduces extra lifecycle state into `WorldTxn` without strengthening correctness on its own. Keep this ticket focused on one robust path: successful journal-to-event commit.

## Files to Touch

- `crates/worldwake-sim/src/world_txn.rs` (modify — add commit and tests)

## Out of Scope

- Any rollback/staging design for world mutations
- Nested transactions
- Multi-event transactions (one WorldTxn = one EventRecord, always)
- Completeness verification (E06EVELOG-009)
- Cause chain traversal (E06EVELOG-008)
- `abort()` / lifecycle-warning APIs that would legitimize non-emitted persistent writes

## Acceptance Criteria

### Tests That Must Pass

1. `commit()` produces an `EventRecord` in the `EventLog` with the correct `event_id`
2. `commit()` returns the assigned `EventId`
3. After commit, `EventLog::get(id)` returns the record with matching fields
4. `target_ids` in the committed record are sorted and deduplicated via `PendingEvent::new`
5. `state_deltas` in the committed record preserve mutation order
6. `tags` in the committed record match accumulated tags
7. `commit()` with empty deltas succeeds (system tick events)
8. After commit, `WorldTxn` is consumed — cannot mutate further (compile-time guarantee)
9. Sequential commits from multiple WorldTxn instances produce sequential EventIds
10. EventLog secondary indices are updated correctly after commit
11. Existing suite: `cargo test --workspace`

### Invariants

1. One WorldTxn commit produces exactly one EventRecord (spec 9.3)
2. Event IDs remain monotonic and gapless after commits
3. Committed deltas are immutable in the log (append-only)
4. `target_ids` are in stable sorted order in the record
5. This ticket must not introduce a first-class API for persisting world mutations without emitting an event

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/world_txn.rs` — commit produces correct record, sequential IDs across multiple commits, canonical target sorting via `PendingEvent`, empty deltas allowed, archive teardown deltas survive commit unchanged, end-to-end create/mutate → commit → verify in log

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`

## Outcome

Outcome amended: 2026-03-09

Implemented:
- `WorldTxn::commit(self, event_log: &mut EventLog) -> EventId` in `crates/worldwake-core/src/world_txn.rs`
- commit-focused tests covering canonical target ordering, empty-delta events, sequential IDs, secondary-index updates, and archive batch preservation

Changed from the original plan:
- kept target canonicalization in `PendingEvent::new(...)` instead of duplicating it in `WorldTxn::commit`
- removed `abort()` and `Drop` warning work from scope because, under the current immediate-mutation architecture, they would add lifecycle surface without improving causal completeness
- the final architecture colocated commit with the authoritative log in `worldwake-core`, which is cleaner than leaving the commit boundary in `worldwake-sim`
