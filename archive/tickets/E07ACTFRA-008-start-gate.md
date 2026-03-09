# E07ACTFRA-008: Start Gate

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — defines action initiation pipeline
**Deps**: E07ACTFRA-003 (ActionDef, ActionDefRegistry), E07ACTFRA-004 (ActionInstance), E07ACTFRA-005 (Handler Registry), E07ACTFRA-006 (KnowledgeView), E07ACTFRA-007 (Affordance)
**Dependency Note**: Completed E07 prerequisites are archived under `archive/tickets/`. For this ticket, see `archive/tickets/E07ACTFRA-003-action-def-action-def-registry.md`, `archive/tickets/E07ACTFRA-004-action-state-action-instance.md`, and `archive/tickets/E07ACTFRA-005-action-handler-function-table-registry.md`.

## Problem

Starting an action requires a strict validation-then-commit sequence: validate actor constraints, check start preconditions, acquire reservations atomically, create an ActionInstance, and emit an action-start event. Failure must return a precise error and emit nothing persistent.

## Assumption Reassessment (2026-03-09)

1. `worldwake-sim` currently provides `ActionDef`, `ActionDefRegistry`, `ActionHandler`, `ActionHandlerRegistry`, `ActionInstance`, `KnowledgeView`, `WorldKnowledgeView`, and affordance-query helpers. There is no start-gate implementation yet.
2. `WorldTxn` does not persist anything by itself. It mutates `World` and records deltas/tags, then becomes an event only when committed into an explicit `EventLog`.
3. Reservation acquisition in core is exposed as `WorldTxn::try_reserve()` / `release_reservation()`, not `create_reservation()`.
4. `EventTag::ActionStarted`, `ReservationId`, `TickRange`, `CauseRef`, and `EventLog` already exist and are the relevant primitives for this ticket.
5. The affordance query from E07ACTFRA-007 pre-validates constraints/preconditions, but the start gate must re-validate against authoritative current state before mutating anything.
6. The current `ActionHandler::on_start` signature takes `&mut WorldTxn`, which would let handlers mutate world state before initialization succeeds. That contradicts this ticket's atomic-failure guarantee, so the handler contract must be tightened first.

## Architecture Check

1. The start gate remains a free function, not a method. It should take explicit registries plus the mutable world/event-log boundary needed to create one auditable start event.
2. Start-time legality must be re-checked against authoritative state. In Phase 1 that means constructing a fresh `WorldKnowledgeView` from `&World` inside the start gate rather than trusting a caller-supplied view snapshot.
3. Reservation acquisition must be atomic: either all required reservations succeed, or all already-acquired reservations are released before the function returns an error.
4. `handler.on_start()` should be a pure initializer that returns `Option<ActionState>` and may not mutate world state. World mutation hooks remain `on_tick`, `on_commit`, and `on_abort`, all through `WorldTxn`.
5. Persistent event emission belongs to the start gate because it is the unit that creates the reservation deltas and the action-start record. That requires an explicit `EventLog` parameter rather than implying hidden persistence inside `WorldTxn`.

## What to Change

### 0. Tighten `ActionHandler::on_start`

Update `worldwake-sim/src/action_handler.rs` so `on_start` becomes a pure initializer:

```rust
pub type ActionStartFn = fn(&ActionInstance) -> Result<Option<ActionState>, ActionError>;
```

Rationale:
- this preserves the ticket's atomic-failure guarantee
- it keeps start-time local-state initialization separate from world mutation
- it still lets handlers seed deterministic `ActionState` before tick/commit work begins

### 1. Create `worldwake-sim/src/start_gate.rs`

Implement:
```rust
pub struct StartActionContext {
    pub cause: CauseRef,
    pub tick: Tick,
}

pub struct StartActionAuthority<'a> {
    pub active_actions: &'a mut BTreeMap<ActionInstanceId, ActionInstance>,
    pub world: &'a mut World,
    pub event_log: &'a mut EventLog,
    pub next_instance_id: &'a mut ActionInstanceId,
}

pub fn start_action(
    affordance: &Affordance,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    authority: StartActionAuthority<'_>,
    context: StartActionContext,
) -> Result<ActionInstanceId, ActionError>
```

Steps:
1. Look up `ActionDef` by `affordance.def_id`
2. Build a fresh authoritative `WorldKnowledgeView` from `world`
3. Re-validate actor constraints against that authoritative view
4. Re-validate start preconditions against that authoritative view
5. Resolve `DurationExpr` to tick count
6. Create a `WorldTxn` scoped to this action start with:
   - `tick = context.tick`
   - `cause = context.cause`
   - `actor_id = Some(affordance.actor)`
   - `place_id = authoritative actor place`
   - `visibility = def.visibility`
   - `witness_data = WitnessData::default()` for Phase 1
7. Acquire all reservations atomically via `WorldTxn::try_reserve()`
   - If any reservation fails, release already-acquired ones and return error
8. Generate `ActionInstanceId` (monotonic increment)
9. Create `ActionInstance` with status `Active`, `remaining_ticks` from duration
10. Call `handler.on_start()` to get initial `ActionState`
11. Add `EventTag::ActionStarted` to the txn, add bound targets, and commit the txn into `event_log`
12. Insert the `ActionInstance` into the authoritative active-action set
13. Return the `ActionInstanceId`

Notes:
- This ticket now inserts started actions directly into the authoritative active-action set instead of leaving lifecycle ownership with the caller.
- Start-gate legality should reuse the pure affordance helpers from E07ACTFRA-007 rather than duplicating constraint/precondition logic.
- Reservation requirements currently bind only by target index, so reservation acquisition should fail closed if a requirement references a missing target binding.

### 2. Update `worldwake-sim/src/lib.rs`

Declare module, re-export `start_action`.

### 3. Strengthen `ActionError` for start-gate failures

The current error surface is too coarse for this ticket's acceptance criteria. Update it so the start gate can report at least:
- missing action definition
- missing action handler
- actor-constraint failure
- start-precondition failure
- reservation conflict

Exact variant naming is up to implementation, but failures should not be collapsed into `InternalError` when the caller can handle them structurally.

## Files to Touch

- `crates/worldwake-sim/src/action_handler.rs` (modify)
- `crates/worldwake-sim/src/action_handler_registry.rs` (modify tests as needed for the tightened `on_start`)
- `crates/worldwake-sim/src/start_gate.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- Tick progression (E07ACTFRA-009)
- Commit validation (E07ACTFRA-009)
- Interrupt/abort (E07ACTFRA-010)
- Scheduler integration (E08)
- Concrete action definitions (later epics)

## Acceptance Criteria

### Tests That Must Pass

1. A valid affordance produces an `ActionInstance` with status `Active`
2. Start gate emits exactly one event with `EventTag::ActionStarted`
3. If actor constraint fails at start time, no event is emitted and error is returned
4. If precondition fails at start time, no event is emitted and error is returned
5. If reservation acquisition fails, no reservations are left dangling and error is returned
6. `ActionInstanceId` increments monotonically across calls
7. The inserted `ActionInstance` has correct `def_id`, `actor`, `targets`, `start_tick`, `remaining_ticks`, `reservation_ids`
8. `handler.on_start()` can initialize `ActionState` but cannot mutate world state
9. Start-gate legality reuses the same pure constraint/precondition evaluators as the affordance query
10. Existing suite: `cargo test --workspace`

### Invariants

1. Spec 9.9: no action starts unless start preconditions are true
2. Spec 9.12: start gate does not branch on `ControlSource`
3. Reservation acquisition is atomic — all or none
4. On failure, no persistent event is emitted, no reservations remain live, and no active action is inserted
5. Start-time local-state initialization is pure; world mutation at start is mediated only by the start gate's `WorldTxn` and authoritative active-action set

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_handler.rs` — `on_start` remains callable as a pure initializer
2. `crates/worldwake-sim/src/action_handler_registry.rs` — registry tests updated for the tightened `on_start` signature
3. `crates/worldwake-sim/src/start_gate.rs` — happy path, constraint failure, precondition failure, reservation failure, monotonic ID assignment, event emission verification, handler-initialized local state, missing definition/handler failures, duplicate active-action ID rejection

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

What actually changed vs. originally planned:

1. Added `crates/worldwake-sim/src/start_gate.rs` with `start_action()`, `StartActionContext`, and `StartActionAuthority`. This kept the public API explicit while making the start-time mutation boundary match the future scheduler-owned authority model.
2. Tightened `ActionHandler::on_start` into a pure initializer. This was the key architectural correction: the previous signature could have mutated world state before start-gate initialization succeeded, which made the ticket's atomic-failure guarantee impossible.
3. Expanded `ActionError` so missing action definitions, missing handlers, constraint failures, precondition failures, and reservation conflicts are reported structurally instead of collapsing into `InternalError`.
4. Reused the existing affordance legality helpers (`evaluate_constraint`, `evaluate_precondition`) for authoritative start-time revalidation instead of duplicating that logic.
5. Post-completion architectural refinement: the start gate no longer returns caller-owned `ActionInstance` state. It inserts directly into the authoritative active-action `BTreeMap<ActionInstanceId, ActionInstance>` and returns the stable instance ID, aligning E07 with the E08 scheduler shape.

Verification:

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
