# E07ACTFRA-008: Start Gate

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — defines action initiation pipeline
**Deps**: E07ACTFRA-003 (ActionDef, ActionDefRegistry), E07ACTFRA-004 (ActionInstance), E07ACTFRA-005 (Handler Registry), E07ACTFRA-006 (KnowledgeView), E07ACTFRA-007 (Affordance)

## Problem

Starting an action requires a strict validation-then-commit sequence: validate actor constraints, check start preconditions, acquire reservations atomically, create an ActionInstance, and emit an action-start event. Failure must return a precise error and emit nothing persistent.

## Assumption Reassessment (2026-03-09)

1. `WorldTxn` in core supports `create_reservation()` and event emission — confirmed from `world_txn.rs`.
2. `EventTag::ActionStarted` exists in core's `event_tag.rs` — confirmed.
3. `ReservationId` and `TickRange` exist in core's `ids.rs` — confirmed.
4. The affordance query from E07ACTFRA-007 pre-validates constraints/preconditions, but the start gate must re-validate (the world may have changed between query and start).

## Architecture Check

1. The start gate is a function, not a method — it takes all dependencies as parameters for testability.
2. Reservation acquisition must be atomic: either all required reservations succeed, or none are created. This prevents partial reservation leaks.
3. On failure, no persistent events are emitted. The function returns a descriptive `ActionError`.
4. The start gate calls `handler.on_start()` after creating the instance, allowing the handler to set initial `ActionState`.

## What to Change

### 1. Create `worldwake-sim/src/start_gate.rs`

Implement:
```rust
pub fn start_action(
    affordance: &Affordance,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    view: &dyn KnowledgeView,
    world: &mut World,
    current_tick: Tick,
    next_instance_id: &mut ActionInstanceId,
) -> Result<ActionInstance, ActionError>
```

Steps:
1. Look up `ActionDef` by `affordance.def_id`
2. Re-validate actor constraints against `view`
3. Re-validate start preconditions against `view`
4. Resolve `DurationExpr` to tick count
5. Acquire all reservations atomically via `WorldTxn`
   - If any reservation fails, release already-acquired ones and return error
6. Generate `ActionInstanceId` (monotonic increment)
7. Create `ActionInstance` with status `Active`, `remaining_ticks` from duration
8. Call `handler.on_start()` to get initial `ActionState`
9. Emit action-start event via `WorldTxn` with `EventTag::ActionStarted`
10. Return the `ActionInstance`

### 2. Update `worldwake-sim/src/lib.rs`

Declare module, re-export `start_action`.

## Files to Touch

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
7. The returned `ActionInstance` has correct `def_id`, `actor`, `targets`, `start_tick`, `remaining_ticks`, `reservation_ids`
8. Action effects mutate world state only through `WorldTxn`
9. Existing suite: `cargo test --workspace`

### Invariants

1. Spec 9.9: no action starts unless start preconditions are true
2. Spec 9.12: start gate does not branch on `ControlSource`
3. Reservation acquisition is atomic — all or none
4. On failure, no persistent state changes occur

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/start_gate.rs` — happy path, constraint failure, precondition failure, reservation failure, monotonic ID assignment, event emission verification

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`
