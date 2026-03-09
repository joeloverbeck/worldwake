# E06EVELOG-010: Staged WorldTxn + Atomic Commit Boundary

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — redesigns the authoritative mutation boundary in `worldwake-core`
**Deps**: `archive/tickets/E06EVELOG-006-world-txn-mutation-journal.md`, `archive/tickets/E06EVELOG-007-world-txn-commit.md`, `archive/tickets/E07ACTFRA-008-start-gate.md`, `archive/tickets/E07ACTFRA-009-tick-progress-commit-validation.md`

## Problem

`WorldTxn` currently mutates `World` eagerly and only later emits an event record when `commit()` is called. That leaves a structural hole in the causal architecture: if code mutates world state through `WorldTxn` and then returns `Err` before commit, authoritative state has changed but the append-only event log has no matching causal record. That is the opposite of what E06 is supposed to guarantee.

The action framework now makes this gap concrete. `on_tick()`, `on_commit()`, and `on_abort()` are allowed to return `Result`, but they also receive a mutable `WorldTxn`. Today that means handler failure can leave partially applied world changes with no committed event. This is not a bug in one caller; it is a weakness in the mutation boundary itself.

## Assumption Reassessment (2026-03-09)

1. The authoritative `WorldTxn` implementation lives in `crates/worldwake-core/src/world_txn.rs`, not in `worldwake-sim` — confirmed from the E06 archive amendments and current code.
2. `WorldTxn` currently applies writes eagerly to `World` and only journals deltas/tags for later `commit(event_log)` — confirmed from current implementation and the archived E06 tickets.
3. The current test suite already encodes eager semantics. For example, `world_txn` tests assert that uncommitted reads can see prior `WorldTxn` mutations, and E07 action tests rely on handlers mutating through a transaction before the final event is emitted.
4. The current architecture has no rollback path. `commit()` consumes the journal into `EventLog`, but dropping a transaction does not restore the world, and the E06 archive explicitly kept rollback/staging out of scope.
5. This means the real architectural invariant today is weaker than the project foundations require: "mutations should go through `WorldTxn`" is true, but "persistent state changes always have a matching causal record" is not yet structurally enforced.
6. Fixing this is more beneficial than keeping the current model. A staged transaction boundary is cleaner, more robust, and more extensible than trying to document or patch around eager mutation failure modes at each caller.
7. `World` already derives `Clone`, which makes a staged transaction implementation materially simpler than the original ticket text assumes: `WorldTxn` can own a staged clone, apply its existing mutation helpers there, and publish the clone atomically on `commit()`. That is cleaner than inventing a parallel per-query staged-view API before the codebase actually needs one.
8. `commit()` does not currently return `Result` and `EventLog::emit(...)` is infallible, so there is no real "failed commit" path to roll back. The meaningful failure boundary is pre-commit staging and handler error/early return.
9. A large share of current tests and setup helpers create state through `WorldTxn` and then drop the transaction without committing it. Those tests are not proving a causal invariant; they are depending on the very eager-mutation leak this ticket is meant to remove.

## Architecture Check

1. The clean design is: `WorldTxn` stages intended mutations, validates against authoritative state plus staged local state, and applies to `World` only inside `commit()`. That makes the event-log append and the world-state mutation part of one atomic boundary.
2. This is cleaner than adding ad hoc "don't error after mutating" rules to handlers. Contracts that rely on every caller remembering a subtle sequencing rule are brittle; the mutation boundary should make the safe path the default path.
3. This is cleaner than adding a fake `abort()` on the current eager model. Dropping an eager journal does not undo anything, so a named abort path would only legitimize state changes without events.
4. This redesign should not introduce backward-compatibility shims. Callers and tests that currently depend on leaked eager state must commit setup transactions or keep their reads inside the transaction.
5. The cleanest implementation in the current codebase is not a per-operation replay journal plus a separate staged query API. It is: `WorldTxn` owns a staged `World` clone, all existing mutation helpers operate on that staged world, immutable reads continue through `Deref<Target = World>` against the staged state, and `commit()` swaps staged state into the authoritative world before emitting the event.
6. The long-term benefit is broader than E07: every future system gets a trustworthy "all-or-nothing + evented" mutation boundary, which is exactly the kind of core architecture worth paying for once.

## What to Change

### 1. Redesign `WorldTxn` as a staged transaction

Replace the current eager model with a staged world snapshot in `crates/worldwake-core/src/world_txn.rs`.

Recommended direction:

```rust
pub struct WorldTxn<'w> {
    world: &'w mut World,
    staged_world: World,
    tick: Tick,
    cause: CauseRef,
    actor_id: Option<EntityId>,
    place_id: Option<EntityId>,
    tags: BTreeSet<EventTag>,
    target_ids: Vec<EntityId>,
    visibility: VisibilitySpec,
    witness_data: WitnessData,
    deltas: Vec<StateDelta>,
}
```

Rules:

1. `staged_world` is cloned from the authoritative world in `WorldTxn::new(...)`.
2. Public mutation helpers keep their current semantic behavior, but operate only on `staged_world` while continuing to append canonical `StateDelta` values in deterministic order.
3. Staging a mutation must not mutate the authoritative `World` immediately.
4. `commit(self, event_log)` must:
   - replace the authoritative world with `staged_world`
   - emit exactly one event record containing the already-built `deltas`
   - leave no path where a handler error or dropped transaction leaks staged state into authoritative world

### 2. Introduce an explicit staged read surface

The ticket originally assumed a new explicit query layer was required. The codebase does not need that extra abstraction yet.

Revised direction:

1. Keep immutable read-through on `WorldTxn`, but point it at `staged_world` rather than authoritative world.
2. Reads inside a transaction must see:
   - authoritative committed world state as of transaction start
   - plus the transaction's own staged writes
3. Reads outside a transaction must continue to see only committed world state.

This preserves a small, honest API: `WorldTxn` reads mean "what would be true if this transaction committed now."

### 3. Rework mutation helpers around staged semantics

Every `WorldTxn` helper that currently mutates eagerly needs to stage instead:

- entity creation helpers
- placement and containment helpers
- ownership/possession/social helpers
- reservation create/release
- lot split/merge
- archive preparation / archive-related wrappers as applicable

Rules:

1. Validation must happen against `staged_world`, not only the committed world.
2. Delta ordering must remain deterministic and match staged operation order.
3. Failed staging must not leave partial staged mutations or deltas behind.
4. Dropping a transaction without `commit()` must leave authoritative `World` unchanged.

### 4. Update call sites that rely on eager mutation

The redesign will change assumptions in existing users, especially:

- `crates/worldwake-sim/src/start_gate.rs`
- `crates/worldwake-sim/src/tick_action.rs`
- future interrupt/abort work in `tickets/E07ACTFRA-010.md`
- any tests that currently inspect world state before commit

Required scope:

1. Remove implicit dependence on eager mutation in action handlers, test fixtures, and setup helpers.
2. Keep transactional reads inside the transaction when code needs to reason about in-flight changes before commit.
3. Keep the external action architecture explicit: no hidden auto-commit behavior.

### 5. Tighten the invariant in docs/tests

Update the relevant archived ticket outcomes or follow-up notes once implemented so the repository no longer documents eager mutation as an accepted architectural compromise.

## Files to Touch

- `crates/worldwake-core/src/world_txn.rs` (modify)
- `crates/worldwake-sim/src/action_handler.rs` (modify tests as needed)
- `crates/worldwake-sim/src/start_gate.rs` (modify)
- `crates/worldwake-sim/src/tick_action.rs` (modify)
- `crates/worldwake-sim/src/world_knowledge_view.rs` (modify tests as needed)
- `archive/tickets/E06EVELOG-006-world-txn-mutation-journal.md` (amend after implementation)
- `archive/tickets/E06EVELOG-007-world-txn-commit.md` (amend after implementation)

## Out of Scope

- Nested transactions
- Concurrent transactions
- Multi-event transactions (`one WorldTxn -> one EventRecord` stays intact)
- Performance tuning beyond what is necessary to keep the design correct
- Temporary compatibility helpers that preserve the old eager API beside the new staged API

## Acceptance Criteria

### Tests That Must Pass

1. Dropping or erroring out of a `WorldTxn` without `commit()` leaves authoritative world state unchanged
2. `commit()` applies all staged writes atomically and emits exactly one matching event record
3. Transaction-local reads can observe the transaction's own staged writes before commit
4. Committed-world reads outside the transaction cannot observe staged writes before commit
5. Action handlers can fail after staging work without leaving world/event divergence
6. Existing suite: `cargo test --workspace`

### Invariants

1. No authoritative world mutation becomes visible outside a transaction before `commit()`
2. No committed world mutation exists without a matching event record
3. One committed `WorldTxn` still yields exactly one `EventRecord`
4. Failed staging or dropped transactions leave no partial authoritative world write and no partial event write

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world_txn.rs` — add staged-read, drop-without-commit, atomic-commit, and failed-commit rollback tests. Rationale: proves the new mutation boundary rather than just individual helper behavior.
2. `crates/worldwake-sim/src/start_gate.rs` — update setup helpers and failure tests that currently depend on dropped setup transactions still mutating the authoritative world. Rationale: validates start-gate behavior against real staged transaction semantics.
3. `crates/worldwake-sim/src/tick_action.rs` — add handler-error tests that stage mutations and then fail. Rationale: this is the concrete E07 failure mode that motivates the redesign.
4. `crates/worldwake-sim/src/action_handler.rs` and `crates/worldwake-sim/src/world_knowledge_view.rs` — update tests that currently read uncommitted setup state after the transaction has been dropped. Rationale: these are direct assumption mismatches exposed by the new boundary.

### Commands

1. `cargo test -p worldwake-core world_txn`
2. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Implemented:

- `WorldTxn` now stages mutations against an internal cloned `World` and only publishes them to authoritative state inside `commit()`
- immutable transaction reads now reflect staged state instead of leaking authoritative eager writes
- added regression coverage proving dropped transactions do not mutate authoritative world and tick-handler errors after staging do not diverge world and event log
- updated E07 and knowledge-view test fixtures to commit setup transactions explicitly instead of relying on dropped transactions mutating state

Changed from the original plan:

- kept `Deref<Target = World>` as the staged read surface instead of introducing a second explicit staged-query API
- did not add commit-time rollback machinery because `commit()` remains infallible; the meaningful correctness boundary is "nothing escapes before commit"
- touched `crates/worldwake-sim/src/affordance_query.rs` in addition to the originally listed files because its tests also relied on dropped setup transactions persisting state
