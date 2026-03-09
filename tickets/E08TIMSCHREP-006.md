# E08TIMSCHREP-006: Per-tick flow — deterministic tick stepping

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — core tick loop in worldwake-sim
**Deps**: E08TIMSCHREP-002 (DeterministicRng), E08TIMSCHREP-003 (InputQueue), E08TIMSCHREP-004 (ControllerState), E08TIMSCHREP-005 (Scheduler)

## Problem

The E08 spec defines a strict 7-step per-tick sequence that is the authoritative simulation driver. This is the heart of determinism: every tick must execute the same phases in the same order, processing actions and systems in deterministic order. Without this, the simulation cannot be replayed or trusted.

## Assumption Reassessment (2026-03-09)

1. `Scheduler` struct (E08TIMSCHREP-005) holds `current_tick`, `active_actions` (BTreeMap), `system_manifest`, `input_queue`
2. `DeterministicRng` (E08TIMSCHREP-002) provides `substream(tick, system_id, seq)` for per-system randomness
3. `ControllerState` (E08TIMSCHREP-004) tracks the controlled entity
4. Action execution functions (`tick_action`, `start_action`, `abort_action`) exist in worldwake-sim — confirmed
5. `EventLog` exists in worldwake-core with `emit()` — confirmed
6. `World` exists in worldwake-core — confirmed

## Architecture Check

1. The tick flow is a free function (or method on a context struct), not a trait — keeps it explicit and auditable
2. System dispatch should remain closed and fixed, matching the closed `SystemId` set. Do not introduce a dynamic registration API for scheduler phases.
3. Use a fixed dispatch table keyed by `SystemId` declaration order rather than a map-like registry. The scheduler should run the manifest order against a compile-time dispatch surface.
4. Systems are plain functions, not trait objects
5. No phase may mutate authoritative state outside event-producing paths — enforced by the API structure where `World` mutations go through `WorldTxn` which emits events

## What to Change

### 1. New type: `SystemFn` and fixed dispatch table

```rust
pub type SystemFn = fn(&mut World, &mut EventLog, &mut DeterministicRng, Tick);

pub struct SystemDispatchTable {
    handlers: Box<[SystemFn]>,
}
```

- `new(handlers: [SystemFn; N]) -> Self` or an equivalent constructor tied to `SystemId::ALL`
- `get(id: SystemId) -> SystemFn`
- `validate_against(manifest: &SystemManifest) -> Result<(), Vec<SystemId>>` if a validation helper is still useful

The important constraint is architectural, not cosmetic:
- no runtime registration order
- no partially populated mutable registry
- no map lookup when the legal system set is already closed in code

### 2. New function: `step_tick`

Implements the spec's 7-step sequence:

```rust
pub fn step_tick(
    world: &mut World,
    event_log: &mut EventLog,
    scheduler: &mut Scheduler,
    controller: &mut ControllerState,
    rng: &mut DeterministicRng,
    systems: &SystemDispatchTable,
) -> TickStepResult { ... }
```

Steps:
1. **Drain inputs** for `current_tick` in `(tick, sequence_no)` order
2. **Apply control-binding changes** (`SwitchControl`) and accepted action requests (`RequestAction` → `start_action`, `CancelAction` → `abort_action`)
3. **Progress active actions** in sorted `ActionInstanceId` order (call `tick_action`)
4. **Validate and commit completed actions** in sorted id order
5. **Run registered systems** in fixed `system_order` from manifest
6. **Emit end-of-tick marker** — a system-level event marking tick completion
7. **Increment `current_tick`**

### 3. New type: `TickStepResult`

Summary of what happened in the tick:
- `tick: Tick` (the tick that was just processed)
- `inputs_processed: u32`
- `actions_started: u32`
- `actions_completed: u32`
- `actions_aborted: u32`
- `events_emitted_count: u32`

## Files to Touch

- `crates/worldwake-sim/src/system_dispatch.rs` (new)
- `crates/worldwake-sim/src/tick_step.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add modules + re-exports)

## Out of Scope

- Implementing any concrete game systems (E09–E12)
- Replay recording/checking during tick steps (E08TIMSCHREP-008/009)
- Save/load (E08TIMSCHREP-011)
- Canonical hashing (E08TIMSCHREP-007)
- The `SimulationState` root struct (E08TIMSCHREP-010)

## Acceptance Criteria

### Tests That Must Pass

1. `step_tick` with empty inputs and no active actions: tick increments, no actions processed
2. `step_tick` drains inputs in `(tick, sequence_no)` order — verified by checking action start order
3. `RequestAction` input creates a new active action via `start_action`
4. `CancelAction` input aborts the targeted action
5. `SwitchControl` input updates `ControllerState`
6. Active actions are ticked in sorted `ActionInstanceId` order — test with 3+ actions, verify call order
7. Completed actions are removed from active set after commit
8. Systems run in manifest order — test with 2+ mock systems that append to a shared log, verify order
9. End-of-tick event is emitted with `CauseRef::SystemTick(current_tick)`
10. `SystemDispatchTable` construction or validation rejects mismatched handler counts / missing manifest coverage
11. Two identical tick sequences (same state, same inputs) produce identical results (determinism smoke test)
12. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. System execution order is fixed by manifest (Spec requirement)
2. Active actions progress in sorted ID order (Spec requirement)
3. No state mutation outside event-producing paths
4. No `HashMap`/`HashSet` in tick step logic

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/system_dispatch.rs` — fixed dispatch-table coverage, lookup, validation
2. `crates/worldwake-sim/src/tick_step.rs` — full tick flow integration tests with mock systems and sample actions

### Commands

1. `cargo test -p worldwake-sim tick_step`
2. `cargo test -p worldwake-sim system_dispatch`
3. `cargo clippy --workspace && cargo test --workspace`
