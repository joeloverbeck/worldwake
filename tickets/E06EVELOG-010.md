# E06EVELOG-010: Staged WorldTxn + Atomic Commit Boundary

**Status**: PENDING
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

## Architecture Check

1. The clean design is: `WorldTxn` stages intended mutations, validates against authoritative state plus staged local state, and applies to `World` only inside `commit()`. That makes the event-log append and the world-state mutation part of one atomic boundary.
2. This is cleaner than adding ad hoc "don't error after mutating" rules to handlers. Contracts that rely on every caller remembering a subtle sequencing rule are brittle; the mutation boundary should make the safe path the default path.
3. This is cleaner than adding a fake `abort()` on the current eager model. Dropping an eager journal does not undo anything, so a named abort path would only legitimize state changes without events.
4. This redesign should not introduce backward-compatibility shims. Callers that depend on eager read-through must be updated to use an explicit staged read surface or to commit earlier. If something breaks, update it.
5. The long-term benefit is broader than E07: every future system gets a trustworthy "all-or-nothing + evented" mutation boundary, which is exactly the kind of core architecture worth paying for once.

## What to Change

### 1. Redesign `WorldTxn` as a staged transaction

Replace the current eager model with a staged operation journal in `crates/worldwake-core/src/world_txn.rs`.

Recommended direction:

```rust
pub struct WorldTxn<'w> {
    world: &'w mut World,
    tick: Tick,
    cause: CauseRef,
    actor_id: Option<EntityId>,
    place_id: Option<EntityId>,
    tags: BTreeSet<EventTag>,
    target_ids: Vec<EntityId>,
    visibility: VisibilitySpec,
    witness_data: WitnessData,
    staged_ops: Vec<TxnOp>,
    staged_deltas: Vec<StateDelta>,
    staged_view: StagedWorldView,
}
```

Rules:

1. `TxnOp` is the authoritative list of intended writes in deterministic order.
2. Public mutation helpers stage both:
   - the semantic delta(s) that will appear in the event log
   - the concrete operation(s) needed to apply those writes at commit time
3. Staging a mutation must not mutate `World` immediately.
4. `commit(self, event_log)` must:
   - validate/apply all staged operations to `World`
   - emit exactly one event record containing the already-built `staged_deltas`
   - leave no path where world state is changed but the event is not emitted

### 2. Introduce an explicit staged read surface

The current `Deref<Target = World>` read-through is tied to eager mutation and should not survive unchanged.

Recommended direction:

1. Replace implicit deref reads with an explicit staged read API:
   - `view(&self) -> impl KnowledgeView` or
   - dedicated staged query helpers
2. Reads inside a transaction must see:
   - authoritative committed world state
   - plus the transaction's own staged writes
3. Reads outside a transaction must continue to see only committed world state.

This is a key design point: the transaction needs an honest "what the world would look like if committed now" surface, not magical mutation of the real world.

### 3. Rework mutation helpers around staged semantics

Every `WorldTxn` helper that currently mutates eagerly needs to stage instead:

- entity creation helpers
- placement and containment helpers
- ownership/possession/social helpers
- reservation create/release
- lot split/merge
- archive preparation / archive-related wrappers as applicable

Rules:

1. Validation must happen against the staged view, not only the committed world.
2. Delta ordering must remain deterministic and match staged operation order.
3. Failed staging must not leave partial staged operations or deltas behind.
4. Failed commit must leave `World` unchanged.

### 4. Update call sites that rely on eager mutation

The redesign will change assumptions in existing users, especially:

- `crates/worldwake-sim/src/start_gate.rs`
- `crates/worldwake-sim/src/tick_action.rs`
- future interrupt/abort work in `tickets/E07ACTFRA-010.md`
- any tests that currently inspect world state before commit

Required scope:

1. Remove implicit dependence on eager mutation in action handlers and tests.
2. Use the staged read surface when code needs to reason about in-flight changes before commit.
3. Keep the external action architecture explicit: no hidden auto-commit behavior.

### 5. Tighten the invariant in docs/tests

Update the relevant archived ticket outcomes or follow-up notes once implemented so the repository no longer documents eager mutation as an accepted architectural compromise.

## Files to Touch

- `crates/worldwake-core/src/world_txn.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify as needed for commit-time apply helpers)
- `crates/worldwake-core/src/world/reservations.rs` (modify as needed for staged validation/apply)
- `crates/worldwake-core/src/verification.rs` (modify if completeness checks need strengthening around staged commit)
- `crates/worldwake-sim/src/start_gate.rs` (modify)
- `crates/worldwake-sim/src/tick_action.rs` (modify)
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
4. Failed staging or failed commit leaves no partial world write and no partial event write

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world_txn.rs` — add staged-read, drop-without-commit, atomic-commit, and failed-commit rollback tests. Rationale: proves the new mutation boundary rather than just individual helper behavior.
2. `crates/worldwake-sim/src/start_gate.rs` — update tests that currently assume world mutations are visible immediately. Rationale: validates start-gate behavior against staged transaction semantics.
3. `crates/worldwake-sim/src/tick_action.rs` — add handler-error tests that stage mutations and then fail. Rationale: this is the concrete E07 failure mode that motivates the redesign.
4. `crates/worldwake-core/src/verification.rs` — strengthen any completeness assertions needed once commit becomes the sole visibility boundary. Rationale: keeps E06's causal-completeness checks aligned with the new architecture.

### Commands

1. `cargo test -p worldwake-core worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
