# E07ACTFRA-010: Interrupt/Abort + ReplanNeeded

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — defines external interruption and replan signaling
**Deps**: E07ACTFRA-004 (ActionInstance, ActionStatus), E07ACTFRA-005 (Handler Registry), E07ACTFRA-002 (Interruptibility)

## Problem

External events (combat, facility destruction, target disappearance) can interrupt or abort actions. Interrupt must respect the action's `Interruptibility` setting. Abort always succeeds. Both must release reservations and emit auditable events. When an action fails or is interrupted, a serializable `ReplanNeeded` record signals the AI planner to generate a new plan.

## Assumption Reassessment (2026-03-09)

1. `Interruptibility` from E07ACTFRA-002 has three variants: NonInterruptible, InterruptibleWithPenalty, FreelyInterruptible.
2. `ActionStatus::Interrupted` and `ActionStatus::Aborted` from E07ACTFRA-001 are the terminal states.
3. `EventTag::ActionAborted` exists in core — confirmed. No separate `ActionInterrupted` tag exists yet; we can use `ActionAborted` for both or add one.
4. `ReplanNeeded` must survive save/load — it must be serializable.
5. `AbortReason` from E07ACTFRA-005 provides the reason payload.

## Architecture Check

1. `interrupt()` checks `Interruptibility` before proceeding. Non-interruptible actions reject the interrupt with an error. This is the only function in the action framework that respects interruptibility.
2. `abort()` always succeeds regardless of interruptibility — it is the unconditional termination path.
3. `ReplanNeeded` is a plain serializable struct, not an event payload. It can be stored in a scheduler-side queue or emitted as an event — the spec allows either. We choose to emit it as part of the abort/interrupt event's state deltas for causal traceability.
4. Both functions release reservations and emit events — no reservation leaks.

## What to Change

### 1. Create `worldwake-sim/src/interrupt_abort.rs`

Implement:
```rust
pub fn interrupt_action(
    instance: &mut ActionInstance,
    def: &ActionDef,
    handler_registry: &ActionHandlerRegistry,
    reason: String,
    world: &mut World,
    current_tick: Tick,
) -> Result<ReplanNeeded, ActionError>
```

Logic:
1. Check `def.interruptibility`:
   - `NonInterruptible` → return error
   - `InterruptibleWithPenalty` → proceed (penalty handling deferred to handler)
   - `FreelyInterruptible` → proceed
2. Set `instance.status = Interrupted`
3. Call `handler.on_abort()` with `AbortReason::Interrupted`
4. Release all reservations via `WorldTxn`
5. Emit event with `EventTag::ActionAborted`
6. Return `ReplanNeeded`

```rust
pub fn abort_action(
    instance: &mut ActionInstance,
    handler_registry: &ActionHandlerRegistry,
    reason: String,
    world: &mut World,
    current_tick: Tick,
) -> ReplanNeeded
```

Logic:
1. Set `instance.status = Aborted`
2. Call `handler.on_abort()` with `AbortReason::ExternalAbort`
3. Release all reservations via `WorldTxn`
4. Emit event with `EventTag::ActionAborted`
5. Return `ReplanNeeded`

### 2. Create `worldwake-sim/src/replan_needed.rs`

Define:
```rust
pub struct ReplanNeeded {
    pub agent: EntityId,
    pub failed_action_def: ActionDefId,
    pub failed_instance: ActionInstanceId,
    pub reason: String,
    pub tick: Tick,
}
```

Must derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 3. Update `worldwake-sim/src/lib.rs`

Declare modules, re-export `interrupt_action`, `abort_action`, `ReplanNeeded`.

## Files to Touch

- `crates/worldwake-sim/src/interrupt_abort.rs` (new)
- `crates/worldwake-sim/src/replan_needed.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- AI replanning logic (E13)
- Scheduler integration for processing ReplanNeeded (E08)
- Concrete interrupt triggers (combat system, etc. — later epics)
- Penalty mechanics for InterruptibleWithPenalty (deferred to handler implementations)

## Acceptance Criteria

### Tests That Must Pass

1. Interrupt on `NonInterruptible` action returns error and does not change status
2. Interrupt on `FreelyInterruptible` action sets status to `Interrupted` and releases reservations
3. Interrupt on `InterruptibleWithPenalty` action sets status to `Interrupted` and releases reservations
4. Abort always succeeds regardless of interruptibility
5. Abort sets status to `Aborted` and releases reservations
6. Both emit events with `EventTag::ActionAborted`
7. Both return a `ReplanNeeded` record (interrupt returns it in `Ok`, abort returns it directly)
8. `ReplanNeeded` survives bincode round-trip
9. `ReplanNeeded` contains correct agent, action def, instance ID, reason, and tick
10. Interrupted/aborted actions stop consuming time immediately (verified by attempting to tick after interrupt/abort)
11. Existing suite: `cargo test --workspace`

### Invariants

1. Interrupt obeys `Interruptibility` — NonInterruptible actions are protected
2. Abort always succeeds — it is unconditional
3. Both release all reservations — no orphan reservations
4. Both emit auditable events
5. `ReplanNeeded` is serializable and survives save/load

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/interrupt_abort.rs` — interrupt with each interruptibility variant, abort always-succeeds, reservation release, event emission
2. `crates/worldwake-sim/src/replan_needed.rs` — trait assertions, bincode round-trip

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`
