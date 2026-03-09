# E08TIMSCHREP-006: Per-tick flow — deterministic tick stepping

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — authoritative tick loop in `worldwake-sim`
**Deps**: E08TIMSCHREP-002 (DeterministicRng), E08TIMSCHREP-003 (InputQueue), E08TIMSCHREP-004 (ControllerState), E08TIMSCHREP-005 (Scheduler)

## Problem

The E08 spec defines a strict 7-step per-tick sequence that is the authoritative simulation driver. That loop does not exist yet. The current code already has deterministic scheduler state, ordered inputs, ordered active actions, action lifecycle helpers, and deterministic RNG substreams, but they are not yet composed into one explicit tick-step entry point.

Without that entry point, determinism is still a set of local guarantees instead of an auditable simulation law.

## Assumption Reassessment (2026-03-09)

1. `Scheduler` already exists in `crates/worldwake-sim/src/scheduler.rs` and owns `current_tick`, `active_actions`, `system_manifest`, `input_queue`, and `next_instance_id`
2. `SystemManifest` already exists and already enforces duplicate-free fixed ordering over the closed `SystemId` set
3. `DeterministicRng` already exists and provides `substream(tick, system_id, seq)`
4. `ControllerState` already exists and exposes `switch_control(from, to) -> Result<(), ControlError>`
5. `start_action`, `tick_action`, and `abort_action` already exist, but they do **not** operate on `Scheduler` directly; they require `ActionDefRegistry`, `ActionHandlerRegistry`, and `ActionExecutionContext` / `ActionExecutionAuthority`
6. `EventLog` already exists in `worldwake-core`, not `worldwake-sim`
7. `CauseRef::SystemTick(Tick)` and `EventTag::System` already exist, so the end-of-tick marker can use the existing event model without inventing a new cause/tag abstraction
8. No system dispatch layer exists yet, and `worldwake-systems` does not yet expose concrete system functions for these `SystemId`s

## Architecture Check

1. The tick flow should be a free function with explicit dependencies. This keeps the authoritative simulation path visible and auditable.
2. The tick-step API must take the existing action registries explicitly. Hiding them behind globals or reconstructing action logic inside `step_tick` would duplicate invariants and weaken the architecture.
3. System dispatch should stay closed over `SystemId`. A compile-time keyed dispatch table is better than a dynamic registration API because the legal system set is already fixed in code.
4. The dispatch surface should use a small execution-context struct rather than a raw multi-argument function type. That keeps the interface explicit today and extensible tomorrow without aliasing or wrapper churn.
5. `Scheduler` should remain the owner of ordering state; `step_tick` should orchestrate it, not duplicate it in parallel collections.
6. End-of-tick emission should use `PendingEvent` directly with `CauseRef::SystemTick(current_tick)` and `EventTag::System`. No new event-log abstraction is justified here.
7. Errors from control switching or action execution should fail loudly through a structured tick-step error type. Silent drops would hide determinism bugs.

## What to Change

### 1. New system dispatch module

Add a closed dispatch surface in `worldwake-sim` for the existing closed `SystemId` set.

```rust
pub type SystemFn = fn(SystemExecutionContext<'_>) -> Result<(), SystemError>;

pub struct SystemExecutionContext<'a> {
    pub world: &'a mut World,
    pub event_log: &'a mut EventLog,
    pub rng: &'a mut DeterministicRng,
    pub tick: Tick,
    pub system_id: SystemId,
}

pub struct SystemDispatchTable {
    handlers: [SystemFn; SystemId::ALL.len()],
}
```

Required properties:
- closed over `SystemId::ALL`
- no runtime registration ordering
- no partially populated mutable registry
- deterministic lookup by `SystemId`

Recommended API:
- `SystemDispatchTable::from_handlers([SystemFn; N]) -> Self`
- `SystemDispatchTable::canonical_noop() -> Self` for current-phase tests while real systems do not exist yet
- `SystemDispatchTable::get(id: SystemId) -> SystemFn`

### 2. New tick-step module

Add an authoritative `step_tick` entry point:

```rust
pub struct TickStepServices<'a> {
    pub action_defs: &'a ActionDefRegistry,
    pub action_handlers: &'a ActionHandlerRegistry,
    pub systems: &'a SystemDispatchTable,
}

pub fn step_tick(
    world: &mut World,
    event_log: &mut EventLog,
    scheduler: &mut Scheduler,
    controller: &mut ControllerState,
    rng: &mut DeterministicRng,
    services: TickStepServices<'_>,
) -> Result<TickStepResult, TickStepError> { ... }
```

The implementation must perform the spec's 7-step sequence against the existing APIs:

1. Drain inputs for `scheduler.current_tick()` in deterministic queue order
2. Apply drained inputs in order:
   - `SwitchControl` uses `controller.switch_control`
   - `RequestAction` resolves to a concrete `Affordance` using `get_affordances` over `WorldKnowledgeView`, then calls `start_action`
   - `CancelAction` calls `abort_action`
3. Progress active actions in sorted `ActionInstanceId` order via `tick_action`
4. Let committed/aborted actions stay removed; continuing actions are reinserted by `tick_action`
5. Run systems in manifest order using deterministic per-system substreams derived from `(current_tick, system_id, sequence_no)`
6. Emit an end-of-tick marker event using the existing event model:
   - `tick = current_tick`
   - `cause = CauseRef::SystemTick(current_tick)`
   - tags include `EventTag::System`
7. Increment `scheduler.current_tick`

### 3. New result and error types

Add a `TickStepResult` summary:
- `tick: Tick`
- `inputs_processed: u32`
- `actions_started: u32`
- `actions_completed: u32`
- `actions_aborted: u32`
- `systems_ran: u32`
- `events_emitted_count: u32`

Add a `TickStepError` enum that composes with current code instead of bypassing it. Minimum cases:
- control-switch failure
- requested affordance not currently available
- action lifecycle failure (`start_action`, `tick_action`, `abort_action`)
- system dispatch / execution failure if needed

## Files to Touch

- `crates/worldwake-sim/src/system_dispatch.rs` (new)
- `crates/worldwake-sim/src/tick_step.rs` (new)
- `crates/worldwake-sim/src/scheduler.rs` (modify — add crate-private action-state accessors for tick orchestration)
- `crates/worldwake-sim/src/system_manifest.rs` (modify — add stable ordinal helper for closed system indexing)
- `crates/worldwake-sim/src/deterministic_rng.rs` (modify — reuse `SystemId` ordinal source)
- `crates/worldwake-sim/src/lib.rs` (modify — add modules + re-exports)

## Out of Scope

- Implementing concrete domain systems in `worldwake-systems` (E09–E12)
- Replay recording/checking during tick steps (E08TIMSCHREP-008/009)
- Save/load (E08TIMSCHREP-011)
- Canonical hashing (E08TIMSCHREP-007)
- The `SimulationState` root struct (E08TIMSCHREP-010)
- Reworking the action lifecycle APIs introduced by earlier tickets

## Architectural Decision Notes

Compared with the original draft, the corrected direction is stronger in three ways:

1. It does not invent a parallel action-execution path. `step_tick` must compose the existing `start_action` / `tick_action` / `abort_action` APIs rather than partially reimplement them.
2. It keeps system dispatch closed and explicit, but avoids a brittle raw function signature by using a context struct. That is more extensible without introducing compatibility wrappers later.
3. It reuses the existing event model for end-of-tick markers. Adding new event abstractions here would increase surface area without improving causality or determinism.

## Acceptance Criteria

### Tests That Must Pass

1. `step_tick` with empty inputs and no active actions increments the tick and emits exactly one end-of-tick system event
2. `step_tick` drains same-tick inputs in `sequence_no` order
3. `RequestAction` starts an action only when the requested `(actor, def_id, targets)` matches a currently available affordance
4. `RequestAction` for an unavailable affordance returns a structured `TickStepError`
5. `CancelAction` aborts the targeted action and removes it from the active set
6. `SwitchControl` updates `ControllerState`
7. A mismatched `SwitchControl { from, .. }` returns a structured `TickStepError`
8. Active actions are ticked in sorted `ActionInstanceId` order
9. Continuing actions remain active; committed and aborted actions do not
10. Systems run in manifest order
11. System execution uses deterministic per-system substreams without mutating the parent RNG stream order
12. The end-of-tick marker uses `CauseRef::SystemTick(current_tick)` and includes `EventTag::System`
13. Two identical runs with the same initial state, seed, inputs, and no-op systems produce identical `TickStepResult`, scheduler state, and event-log records
14. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Input handling order is determined solely by `InputQueue`
2. Active-action progression order is determined solely by `BTreeMap<ActionInstanceId, ActionInstance>`
3. System order is determined solely by `SystemManifest`
4. No `HashMap` / `HashSet` ordering is introduced into tick stepping
5. The tick step does not bypass `WorldTxn`-based action mutation paths

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/system_dispatch.rs` — closed dispatch lookup and canonical no-op coverage
2. `crates/worldwake-sim/src/tick_step.rs` — deterministic input handling, action lifecycle orchestration, system ordering, end-of-tick emission, and determinism smoke tests

### Commands

1. `cargo test -p worldwake-sim tick_step`
2. `cargo test -p worldwake-sim system_dispatch`
3. `cargo test -p worldwake-sim`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

Completed: 2026-03-09
Outcome amended: 2026-03-09

What actually changed:
- added `system_dispatch.rs` with a closed `SystemDispatchTable`, `SystemExecutionContext`, and `SystemError`
- added `tick_step.rs` with `step_tick`, `TickStepServices`, `TickStepResult`, and `TickStepError`
- wired `step_tick` through the existing action lifecycle APIs instead of duplicating action logic
- refined the scheduler integration so tick stepping no longer reaches into scheduler internals directly; `Scheduler` now owns the action-set execution handoff via scheduler-side execution methods and a small scheduler runtime bundle
- emitted end-of-tick markers through the existing `PendingEvent` / `EventLog` model using `CauseRef::SystemTick` and `EventTag::System`
- strengthened invariants by validating that `CancelAction.actor` actually owns the targeted active action

Deviations from the original plan:
- `step_tick` takes a `TickStepServices` bundle instead of three separate service arguments; this kept the authoritative API explicit while satisfying strict `clippy` limits on argument count and keeping the function easier to extend
- `SystemId` gained a stable `ordinal()` helper so deterministic RNG and dispatch indexing share one source of truth instead of duplicating slot mappings

Verification results:
- `cargo test -p worldwake-sim tick_step`
- `cargo test -p worldwake-sim`
- `cargo clippy --workspace`
- `cargo test --workspace`
