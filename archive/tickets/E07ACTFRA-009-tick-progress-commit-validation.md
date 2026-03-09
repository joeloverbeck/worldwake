# E07ACTFRA-009: Tick Progress + Commit Validation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — defines active-action ticking, terminal transitions, and commit-time legality
**Deps**: E07ACTFRA-004 (ActionInstance), E07ACTFRA-005 (Handler Registry), E07ACTFRA-008 (Start Gate)
**Dependency Note**: Completed E07 prerequisites are archived under `archive/tickets/`. For this ticket, see `archive/tickets/E07ACTFRA-004-action-state-action-instance.md`, `archive/tickets/E07ACTFRA-005-action-handler-function-table-registry.md`, and `archive/tickets/E07ACTFRA-008-start-gate.md`.

## Problem

Active actions need one deterministic tick function that advances time, lets the handler perform tick-time work through `WorldTxn`, and then either keeps the action active or drives it into commit validation. Commit legality must be re-checked on authoritative state at the exact completion boundary. If commit legality fails, the action must abort cleanly, release reservations, emit an auditable abort event, and leave no terminal instance inside the authoritative active-action set.

## Assumption Reassessment (2026-03-09)

1. `ActionInstance` stores only `def_id`, not `ActionDef` or `ActionHandlerId`, so tick-time dispatch must resolve `def_id -> ActionDef -> ActionHandler` through registries rather than taking `&ActionDef` directly.
2. `WorldTxn` mutates `World` immediately but only becomes part of the append-only causal record when explicitly committed into an `EventLog`. Any tick-time world mutation performed by `handler.on_tick()` therefore requires explicit event-log persistence in this ticket.
3. `EventTag::ActionCommitted` and `EventTag::ActionAborted` already exist in core. Successful commit should also apply the action definition's `causal_event_tags`; abort should not pretend the action committed successfully.
4. The authoritative running-action store today is `BTreeMap<ActionInstanceId, ActionInstance>`, and the current architecture treats it as the active set. Terminal actions therefore should be removed from that map on commit/abort rather than left behind with a terminal status.
5. `DurationExpr::Fixed(0)` is already legal in the type model, so ticking must not blindly decrement `remaining_ticks` into underflow. A zero-remaining active action must go straight to commit validation on its next tick.
6. Commit-condition evaluation should reuse the existing pure `evaluate_precondition()` helper against a fresh authoritative `WorldKnowledgeView`, just as the start gate reused the affordance legality helpers for authoritative revalidation.

## Architecture Check

1. The lifecycle API should stay aligned with the start gate: a free function over explicit registries plus an explicit authority boundary (`active_actions`, `world`, `event_log`). This keeps persistence and mutation ownership honest and scheduler-friendly.
2. `tick_action()` should take an `ActionInstanceId`, not `&mut ActionInstance`. The authoritative action set owns instances; the tick path should resolve, mutate, and remove/reinsert them in one place instead of letting callers hold ad hoc mutable aliases.
3. Invalid lifecycle use should return structured `ActionError` values, not `assert!`. This path is part of the legal action pipeline, not internal test-only scaffolding.
4. `handler.on_tick()` should run inside the same `WorldTxn` that will later be committed as either:
   - a continuing tick event if it actually mutated world state, or
   - the final commit/abort event if the action terminates this tick.
   This keeps append-only causality intact and prevents tick-time mutations from escaping the event log.
5. Successful commit must emit `EventTag::ActionCommitted` plus `def.causal_event_tags`. Both successful commit and failed commit must release reservations through `WorldTxn`; failed commit must additionally call `handler.on_abort()` and emit `EventTag::ActionAborted`.

## What to Change

### 1. Create `worldwake-sim/src/tick_action.rs`

Implement:
```rust
pub struct TickActionContext {
    pub cause: CauseRef,
    pub tick: Tick,
}

pub struct TickActionAuthority<'a> {
    pub active_actions: &'a mut BTreeMap<ActionInstanceId, ActionInstance>,
    pub world: &'a mut World,
    pub event_log: &'a mut EventLog,
}

pub enum TickOutcome {
    Continuing,
    Committed,
    Aborted { reason: AbortReason },
}

pub fn tick_action(
    instance_id: ActionInstanceId,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    authority: TickActionAuthority<'_>,
    context: TickActionContext,
) -> Result<TickOutcome, ActionError>
```

Required flow:

1. Remove or otherwise uniquely borrow the target `ActionInstance` from the authoritative active-action set
2. Resolve `ActionDef` from `instance.def_id`, then resolve the handler from `def.handler`
3. Reject ticking a non-`Active` instance with a structured `ActionError`
4. Build a `WorldTxn` scoped to this tick using:
   - `tick = context.tick`
   - `cause = context.cause`
   - `actor_id = Some(instance.actor)`
   - `place_id = authoritative actor place`
   - `visibility = def.visibility`
   - `witness_data = WitnessData::default()` for Phase 1
5. If `remaining_ticks > 0`, decrement it exactly once
6. Call `handler.on_tick()`
7. If `handler.on_tick()` returns `Complete`, or if `remaining_ticks == 0` after the decrement / zero-duration check:
   - build a fresh authoritative `WorldKnowledgeView`
   - re-evaluate `def.commit_conditions`
   - if all pass:
     - call `handler.on_commit()`
     - release all reservations via `WorldTxn`
     - add `EventTag::ActionCommitted`
     - add every tag from `def.causal_event_tags`
     - add all action targets to the txn
     - commit the txn into `event_log`
     - do **not** reinsert the instance into `active_actions`
     - return `TickOutcome::Committed`
   - if any fail:
     - call `handler.on_abort()` with `AbortReason::CommitConditionFailed(...)`
     - release all reservations via `WorldTxn`
     - add `EventTag::ActionAborted`
     - add all action targets to the txn
     - commit the txn into `event_log`
     - do **not** reinsert the instance into `active_actions`
     - return `TickOutcome::Aborted { reason }`
8. Otherwise:
   - if the tick txn recorded any deltas or tags, add all action targets and commit it into `event_log`
   - reinsert the updated instance into `active_actions`
   - return `TickOutcome::Continuing`

Notes:
- The function should not emit empty no-op events just because time advanced.
- Commit/abort paths own reservation cleanup so the active set never retains a terminal action with live reservations.
- Continue vs. terminal is decided after handler tick logic runs, not before, so handlers can finish early by returning `ActionProgress::Complete`.

### 2. Strengthen `ActionError` for lifecycle execution

The current error surface does not cleanly describe tick-time misuse. Expand it so this ticket can report at least:
- unknown action instance
- missing action definition
- missing action handler
- invalid action status for ticking
- commit-condition failure reason (as abort payload, not necessarily as the top-level returned error)

Exact variant naming is up to implementation, but these cases should not collapse into `InternalError`.

### 3. Update `worldwake-sim/src/lib.rs`

Declare module, re-export `tick_action`, `TickActionAuthority`, `TickActionContext`, and `TickOutcome`.

## Files to Touch

- `crates/worldwake-sim/src/action_handler.rs` (modify `ActionError` if needed)
- `crates/worldwake-sim/src/lib.rs` (modify)
- `crates/worldwake-sim/src/tick_action.rs` (new)

## Out of Scope

- External interrupt / abort entry points (E07ACTFRA-010)
- `ReplanNeeded` transport or scheduler integration (E07ACTFRA-010 / E08)
- Concrete action definitions or concrete gameplay handlers (later epics)
- Refactoring the start gate into a shared lifecycle module unless needed to avoid duplication in touched code

## Acceptance Criteria

### Tests That Must Pass

1. An active action with `remaining_ticks = 3` decrements to 2 after one tick and stays in the active set
2. An action with `remaining_ticks = 1` commits successfully when commit conditions pass
3. A zero-duration active action (`remaining_ticks = 0`) reaches commit validation without underflow
4. **T06**: An action whose commit conditions fail aborts cleanly — reservations are released, `EventTag::ActionAborted` is emitted, and the instance is removed from the active set
5. `handler.on_tick()` is called once per tick attempt
6. `handler.on_commit()` is called exactly once on successful commit
7. `handler.on_abort()` is called on commit failure
8. Successful commit emits `EventTag::ActionCommitted` plus the action definition's `causal_event_tags`
9. Continuing ticks that mutate world state are persisted to `EventLog`; pure no-op continuing ticks do not emit empty events
10. Ticking a missing or non-active instance returns a structured error
11. Existing suite: `cargo test --workspace`

### Invariants

1. Spec 9.9: no action commits unless commit conditions are true at commit time
2. Remaining ticks decrement deterministically without underflow
3. Executable dispatch continues to derive from `def_id -> ActionDef -> handler`
4. Terminal actions are removed from the authoritative active-action set
5. Reservations are always released on commit or abort
6. Tick-time world mutations do not bypass the append-only event log

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_action.rs` — decrement path, zero-duration path, commit success, commit failure -> abort, handler callback verification, active-set removal/reinsertion, event emission, reservation release, and no-op tick event suppression
2. `crates/worldwake-sim/src/action_handler.rs` — trait assertions updated if `ActionError` gains lifecycle variants

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

What actually changed vs. originally planned:

1. Added `crates/worldwake-sim/src/tick_action.rs` with `tick_action()`, `TickActionAuthority`, `TickActionContext`, and `TickOutcome`, aligned to the existing authoritative active-action map rather than introducing a caller-owned `&mut ActionInstance` API.
2. Expanded `ActionError` with structured lifecycle failures for unknown action instances and invalid action status instead of relying on assertions or collapsing those cases into `InternalError`.
3. Successful commit now releases reservations, emits `EventTag::ActionCommitted`, and applies `ActionDef.causal_event_tags`. Failed commit calls `on_abort()`, releases reservations, and emits `EventTag::ActionAborted`.
4. Continuing ticks now persist tick-time world mutations to `EventLog` when `handler.on_tick()` actually mutates world state, while pure no-op continuation does not emit an empty event.
5. The implementation explicitly removes terminal instances from the authoritative active-action set and only reinserts true continuations, keeping the active-action boundary honest for E08 scheduler work.
6. The test plan was strengthened beyond the original ticket to cover zero-duration safety once an action instance exists.

Verification:

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

Outcome amended: 2026-03-09

1. Follow-up architectural refinement in `start_gate` now treats zero-duration actions as having no reservation interval, so they can start cleanly even when they declare reservation requirements.
2. This keeps the type model honest: `DurationExpr::Fixed(0)` remains legal without forcing the start gate to fabricate an invalid non-empty `TickRange`.
