# E07ACTFRA-010: External Interrupt/Abort + ReplanNeeded

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds external termination entry points and structured replan records
**Deps**: E07ACTFRA-002 (Interruptibility), E07ACTFRA-004 (ActionInstance), E07ACTFRA-005 (Handler Registry), E07ACTFRA-009 (Tick Progress + Commit Validation)
**Dependency Note**: Completed E07 prerequisites are archived under `archive/tickets/`. For this ticket, see `archive/tickets/E07ACTFRA-002-supporting-semantic-types.md`, `archive/tickets/E07ACTFRA-004-action-state-action-instance.md`, `archive/tickets/E07ACTFRA-005-action-handler-function-table-registry.md`, and `archive/tickets/E07ACTFRA-009-tick-progress-commit-validation.md`.

## Problem

Active actions already have deterministic start and tick/commit lifecycle entry points. What is still missing is an authoritative way for external world causes to terminate an in-flight action before natural completion. Interrupt must respect `Interruptibility`; abort must remain unconditional. Both must cleanly unwind reservations, invoke handler cleanup, emit auditable events, and return a serializable replan record that later scheduler/AI work can consume.

## Assumption Reassessment (2026-03-09)

1. There is no `specs/E07ACTFRA-010.md`; the governing epic reference is [`archive/specs/E07-action-framework.corrected.md`](/home/joeloverbeck/projects/worldwake/archive/specs/E07-action-framework.corrected.md).
2. The current action lifecycle is already centered on authoritative free functions over registries plus an authority boundary:
   - [`start_action()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs)
   - [`tick_action()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs)
   New external interrupt/abort work should extend that lifecycle, not create a second parallel architecture.
3. `tick_action()` already owns terminal abort-on-commit-failure behavior, reservation release, and event emission. Any new external termination path that reimplements those rules separately would be architecture drift.
4. `WorldTxn` already provides the correct mutation boundary for reservation release and event emission. The event log does not currently support arbitrary typed payloads beyond the existing event record fields, so `ReplanNeeded` cannot be “stored in event state deltas” without first redesigning the event schema.
5. `AbortReason` already exists and is serializable. Using it directly inside `ReplanNeeded` is cleaner and more extensible than collapsing reasons back to raw strings.
6. `EventTag::ActionAborted` exists. There is currently no `EventTag::ActionInterrupted`, but external interrupt is semantically distinct from external abort. Reusing `ActionAborted` for interrupts would blur those outcomes and weaken auditability.
7. The authoritative active-action store is still `BTreeMap<ActionInstanceId, ActionInstance>`. External termination should remove terminal instances from that map just like `tick_action()` already does.

## Architecture Check

1. The clean design is to introduce a shared internal terminal-transition helper and reuse it from both:
   - commit-failure abort in [`tick_action.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs)
   - new external interrupt/abort entry points
   This keeps reservation cleanup, handler cleanup, event tags, target propagation, and replan record construction under one source of truth.
2. External termination entry points should take `ActionInstanceId`, not `&mut ActionInstance`. The authoritative active-action map owns instances, and lifecycle transitions should preserve that ownership boundary.
3. `ReplanNeeded` should be a serializable return value, not an event-log surrogate payload. E08 can decide how pending replan records are queued/persisted at the scheduler layer.
4. Interrupt and abort should remain separate operations with no aliasing:
   - interrupt: legality-gated by `Interruptibility`
   - abort: unconditional termination
5. Auditability is stronger if interrupt and abort emit different event tags. Add `EventTag::ActionInterrupted` rather than overloading `ActionAborted` for both.
6. The current architecture in [`tick_action.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs) is directionally correct, but its terminal abort path is too local for upcoming lifecycle expansion. This ticket should factor that shared logic once instead of cloning it into a second file.

## What to Change

### 1. Add `ReplanNeeded`

Create `crates/worldwake-sim/src/replan_needed.rs`:

```rust
pub struct ReplanNeeded {
    pub agent: EntityId,
    pub failed_action_def: ActionDefId,
    pub failed_instance: ActionInstanceId,
    pub reason: AbortReason,
    pub tick: Tick,
}
```

Must derive:
- `Clone`
- `Debug`
- `Eq`
- `PartialEq`
- `Serialize`
- `Deserialize`

Rationale:
- preserves structured failure semantics
- survives save/load
- avoids lossy string re-parsing later

### 2. Add a shared terminal-transition helper

Create an internal lifecycle helper module in `worldwake-sim` that owns terminal action transitions. Exact module naming is implementation-defined, but it should centralize:

1. setting terminal status
2. calling `handler.on_abort()` or `handler.on_commit()`
3. releasing reservations through `WorldTxn`
4. adding the correct event tag(s)
5. adding all bound targets
6. committing the transaction
7. constructing `ReplanNeeded` for failed/interrupted paths

This helper must be reused by both:
- commit-failure abort inside `tick_action()`
- new external interrupt/abort entry points

### 3. Add external interrupt/abort entry points

Create `crates/worldwake-sim/src/interrupt_abort.rs`:

```rust
pub struct InterruptActionContext {
    pub cause: CauseRef,
    pub tick: Tick,
}

pub struct InterruptActionAuthority<'a> {
    pub active_actions: &'a mut BTreeMap<ActionInstanceId, ActionInstance>,
    pub world: &'a mut World,
    pub event_log: &'a mut EventLog,
}

pub fn interrupt_action(
    instance_id: ActionInstanceId,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    authority: InterruptActionAuthority<'_>,
    context: InterruptActionContext,
    reason: String,
) -> Result<ReplanNeeded, ActionError>
```

Required flow:

1. Remove or uniquely borrow the target instance from the authoritative active-action set
2. Resolve `ActionDef` and `ActionHandler` from registries
3. Reject non-`Active` instances with structured `ActionError`
4. If `def.interruptibility == NonInterruptible`, reinsert the instance unchanged and return a structured error
5. Otherwise:
   - build a `WorldTxn` with actor/place/visibility matching the action definition
   - terminate via the shared helper using:
     - `ActionStatus::Interrupted`
     - `AbortReason::Interrupted(reason)`
     - `EventTag::ActionInterrupted`
   - do not reinsert the instance
   - return the resulting `ReplanNeeded`

Also implement:

```rust
pub fn abort_action(
    instance_id: ActionInstanceId,
    registry: &ActionDefRegistry,
    handler_registry: &ActionHandlerRegistry,
    authority: InterruptActionAuthority<'_>,
    context: InterruptActionContext,
    reason: String,
) -> Result<ReplanNeeded, ActionError>
```

Required flow:

1. Same authoritative lookup/removal rules as `interrupt_action()`
2. Ignore `Interruptibility`
3. Terminate via the shared helper using:
   - `ActionStatus::Aborted`
   - `AbortReason::ExternalAbort(reason)`
   - `EventTag::ActionAborted`
4. Do not reinsert the instance
5. Return `ReplanNeeded`

### 4. Update `tick_action()` to reuse the shared abort path and surface replan data

Modify [`crates/worldwake-sim/src/tick_action.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs):

1. Replace the local commit-failure abort branch with the shared terminal-transition helper
2. Extend `TickOutcome::Aborted` so it carries the `ReplanNeeded` record produced by commit-condition failure
3. Preserve all current behavior for successful commit and continuing ticks

### 5. Add `EventTag::ActionInterrupted`

Modify [`crates/worldwake-core/src/event_tag.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/event_tag.rs):

1. Add `ActionInterrupted`
2. Update canonical variant-list and bincode round-trip tests

### 6. Update `worldwake-sim/src/lib.rs`

Re-export:
- `abort_action`
- `interrupt_action`
- `InterruptActionAuthority`
- `InterruptActionContext`
- `ReplanNeeded`

## Files to Touch

- `crates/worldwake-core/src/event_tag.rs`
- `crates/worldwake-sim/src/lib.rs`
- `crates/worldwake-sim/src/tick_action.rs`
- `crates/worldwake-sim/src/interrupt_abort.rs` (new)
- `crates/worldwake-sim/src/replan_needed.rs` (new)
- internal lifecycle helper module in `crates/worldwake-sim/src/` (new)

## Out of Scope

- E08 scheduler ownership/persistence of pending replan queues
- E13 planner consumption of `ReplanNeeded`
- Concrete interrupt triggers from combat, trade, politics, etc.
- Penalty mechanics for `InterruptibleWithPenalty`
- Broader action-runner or scheduler refactors beyond the shared termination helper needed here

## Acceptance Criteria

### Tests That Must Pass

1. Interrupt on `NonInterruptible` returns a structured error, leaves status unchanged, and keeps the instance in the active set
2. Interrupt on `FreelyInterruptible` removes the instance from the active set, releases reservations, emits `EventTag::ActionInterrupted`, and returns `ReplanNeeded`
3. Interrupt on `InterruptibleWithPenalty` behaves like an interrupt now, with penalty mechanics still deferred
4. Abort always succeeds regardless of interruptibility
5. Abort removes the instance from the active set, releases reservations, emits `EventTag::ActionAborted`, and returns `ReplanNeeded`
6. Both interrupt and abort call `handler.on_abort()` with the correct `AbortReason`
7. `ReplanNeeded` survives bincode round-trip
8. `ReplanNeeded` contains the correct agent, action def id, instance id, structured reason, and tick
9. Commit-condition failure in `tick_action()` now returns `TickOutcome::Aborted { replan: ... }`
10. Interrupted or aborted actions stop consuming time immediately because they are removed from the authoritative active set
11. Existing suite: `cargo test --workspace`

### Invariants

1. Interrupt obeys `Interruptibility`
2. Abort is unconditional
3. Interrupt and abort remain distinct, non-aliased terminal outcomes
4. All failed/interrupted terminal paths release reservations
5. Terminal transitions stay centralized under one lifecycle helper
6. `ReplanNeeded` is serializable and scheduler-ready without abusing the event-log schema

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_tag.rs` — canonical variant list now includes `ActionInterrupted`
2. `crates/worldwake-sim/src/replan_needed.rs` — trait assertions and bincode round-trip
3. `crates/worldwake-sim/src/interrupt_abort.rs` — interruptibility gating, unconditional abort, reservation release, active-set removal, event emission, handler callback reasons, and returned `ReplanNeeded`
4. `crates/worldwake-sim/src/tick_action.rs` — commit-condition failure now returns `ReplanNeeded` in the abort outcome while preserving prior cleanup behavior

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

What actually changed vs. originally planned:

1. Corrected the ticket away from a duplicate lifecycle design and implemented a shared failed-transition helper reused by both commit-failure aborts and new external interrupt/abort entry points.
2. Added `ReplanNeeded` as a structured serializable record that carries `AbortReason` directly, instead of trying to smuggle planner data through event-log deltas.
3. Added distinct external lifecycle APIs in `crates/worldwake-sim/src/interrupt_abort.rs` and surfaced `ReplanNeeded` from both those APIs and `tick_action()` abort outcomes.
4. Added `EventTag::ActionInterrupted` so interrupts remain auditable without aliasing them to aborts.
5. Strengthened tests around interruptibility gating, active-set removal, reservation cleanup, event tagging, and replan serialization.
6. Follow-up lifecycle cleanup consolidated the previously split start/tick/interrupt authority-context structs into a shared action-execution surface, reducing duplicate API shapes without adding compatibility aliases.

## Verification

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
