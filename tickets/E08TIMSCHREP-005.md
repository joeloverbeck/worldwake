# E08TIMSCHREP-005: Scheduler struct

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: E08TIMSCHREP-001 (SystemId/SystemManifest), E08TIMSCHREP-003 (InputQueue)

## Problem

The scheduler is the central coordination point for the tick loop. It owns the current tick, active action set, system execution order, and input queue. Without a dedicated `Scheduler` struct, this state would be scattered, making save/load and replay fragile.

## Assumption Reassessment (2026-03-09)

1. `ActionInstance` exists with `ActionInstanceId` key — confirmed in `action_instance.rs`
2. `BTreeMap` is required for active actions (deterministic iteration) — confirmed by deterministic data policy
3. `SystemManifest` from E08TIMSCHREP-001 will provide fixed system order
4. `InputQueue` from E08TIMSCHREP-003 will provide deterministic input draining
5. No scheduler exists yet — confirmed

## Architecture Check

1. `Scheduler` is a data-owning struct, not a trait — it holds `current_tick`, `active_actions`, `system_manifest`, `input_queue`, and `next_instance_id`
2. Active actions are stored in `BTreeMap<ActionInstanceId, ActionInstance>` — iteration is always sorted by ID, satisfying the spec's "sorted `ActionInstanceId` order" requirement
3. The tick-stepping logic is NOT in this ticket — this ticket is the struct and its accessors. E08TIMSCHREP-006 adds the per-tick flow.

## What to Change

### 1. New type: `Scheduler`

```rust
pub struct Scheduler {
    current_tick: Tick,
    active_actions: BTreeMap<ActionInstanceId, ActionInstance>,
    system_manifest: SystemManifest,
    input_queue: InputQueue,
    next_instance_id: ActionInstanceId,
}
```

Derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 2. Accessor and mutation methods

- `new(system_manifest: SystemManifest) -> Self` — starts at `Tick(0)`
- `new_with_tick(tick: Tick, system_manifest: SystemManifest) -> Self`
- `current_tick(&self) -> Tick`
- `active_actions(&self) -> &BTreeMap<ActionInstanceId, ActionInstance>`
- `system_manifest(&self) -> &SystemManifest`
- `input_queue(&self) -> &InputQueue`
- `input_queue_mut(&mut self) -> &mut InputQueue`
- `allocate_instance_id(&mut self) -> ActionInstanceId` — monotonic
- `insert_action(&mut self, instance: ActionInstance)` — adds to active set
- `remove_action(&mut self, id: ActionInstanceId) -> Option<ActionInstance>`
- `increment_tick(&mut self)` — advances `current_tick` by 1

## Files to Touch

- `crates/worldwake-sim/src/scheduler.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-export)

## Out of Scope

- The per-tick flow / step logic (E08TIMSCHREP-006)
- Replay state or checkpoint recording (E08TIMSCHREP-008)
- RNG integration (E08TIMSCHREP-002 provides the type; E08TIMSCHREP-006 wires it)
- System dispatch (mapping `SystemId` to actual functions — E08TIMSCHREP-006)

## Acceptance Criteria

### Tests That Must Pass

1. `Scheduler` satisfies `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
2. `Scheduler::new` starts at `Tick(0)` with empty active actions and empty input queue
3. `allocate_instance_id` returns monotonically increasing IDs
4. `insert_action` adds to active set; `remove_action` removes it
5. `active_actions()` iteration order matches sorted `ActionInstanceId` (BTreeMap guarantee)
6. `increment_tick` advances tick by 1
7. Bincode round-trip: serialize scheduler with active actions and input events, deserialize, verify equality
8. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Active actions are in `BTreeMap` — iteration order is always deterministic
2. `next_instance_id` is monotonically increasing — no reuse
3. No `HashMap` or `HashSet` in the scheduler

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/scheduler.rs` (inline `#[cfg(test)]`) — construction, action lifecycle, tick advancement, ID allocation, serialization

### Commands

1. `cargo test -p worldwake-sim scheduler`
2. `cargo clippy --workspace && cargo test --workspace`
