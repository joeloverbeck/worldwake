# E08TIMSCHREP-005: Scheduler struct

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: None (E08TIMSCHREP-001 and E08TIMSCHREP-003 are already implemented)

## Problem

The scheduler is the central coordination point for the tick loop. The underlying deterministic primitives now exist, but their authoritative state is still scattered across call sites as loose values (`current_tick`, `active_actions`, `next_instance_id`, manifest, queue). That shape is brittle for save/load, replay, and the upcoming per-tick flow because it makes scheduler state easy to partially pass around or accidentally desynchronize.

## Assumption Reassessment (2026-03-09)

1. `ActionInstance` exists with `ActionInstanceId` key — confirmed in `action_instance.rs`
2. `BTreeMap` is required for active actions (deterministic iteration) — confirmed by deterministic data policy and by the existing action execution helpers
3. `SystemManifest` already exists in `system_manifest.rs` and already enforces duplicate-free fixed ordering
4. `InputQueue` already exists in `input_queue.rs` and already provides deterministic sequence allocation and per-tick draining
5. `ControllerState` and `DeterministicRng` also already exist, but they are out of scope for this ticket
6. No scheduler exists yet — confirmed

## Architecture Check

1. `Scheduler` is a data-owning struct, not a trait — it holds `current_tick`, `active_actions`, `system_manifest`, `input_queue`, and `next_instance_id`
2. `Scheduler` should own `SystemManifest`, not duplicate a raw `Vec<SystemId>`. The manifest already centralizes the ordering invariant and rejects duplicates, so re-expressing that invariant in a second representation would weaken the architecture instead of improving it.
3. Active actions are stored in `BTreeMap<ActionInstanceId, ActionInstance>` — iteration is always sorted by ID, satisfying the spec's "sorted `ActionInstanceId` order" requirement
4. ID and tick advancement should use checked arithmetic. Silent wraparound would corrupt replay determinism and save/load integrity.
5. The tick-stepping logic is NOT in this ticket — this ticket is the scheduler state container and its narrow mutation API. E08TIMSCHREP-006 adds the per-tick flow.

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
- `allocate_instance_id(&mut self) -> ActionInstanceId` — monotonic, panics on overflow
- `insert_action(&mut self, instance: ActionInstance)` — adds to active set
- `remove_action(&mut self, id: ActionInstanceId) -> Option<ActionInstance>`
- `increment_tick(&mut self)` — advances `current_tick` by 1, panics on overflow

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
2. `Scheduler::new` starts at `Tick(0)` with empty active actions, empty input queue, and `next_instance_id = ActionInstanceId(0)`
3. `new_with_tick` preserves the provided tick while still starting with empty scheduler-owned collections
4. `allocate_instance_id` returns monotonically increasing IDs and preserves the invariant across serialization round-trips
5. `insert_action` adds to active set; `remove_action` removes it
6. `active_actions()` iteration order matches sorted `ActionInstanceId` (BTreeMap guarantee)
7. `increment_tick` advances tick by 1
8. Overflow edges fail loudly: instance-id allocation overflow and tick overflow panic rather than wrapping
9. Bincode round-trip: serialize scheduler with active actions and input events, deserialize, verify equality
10. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Active actions are in `BTreeMap` — iteration order is always deterministic
2. `next_instance_id` is monotonically increasing — no reuse
3. No `HashMap` or `HashSet` in the scheduler
4. Scheduler-owned counters never wrap silently

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/scheduler.rs` (inline `#[cfg(test)]`) — construction, action lifecycle, tick advancement, ID allocation, serialization

### Commands

1. `cargo test -p worldwake-sim scheduler`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

Implemented the new `worldwake-sim::scheduler` module and re-exported `Scheduler` from the crate root.

Compared with the original ticket text:
- The ticket assumptions were corrected first: `SystemManifest` and `InputQueue` were already implemented, so this work integrated existing deterministic primitives instead of waiting on them.
- The final implementation kept `SystemManifest` as the owned system-order representation rather than duplicating `Vec<SystemId>`, which is cleaner and preserves the duplicate-check invariant in one place.
- The test scope was strengthened beyond the original draft to cover overflow edges for both `next_instance_id` and `current_tick`, so the scheduler now fails loudly instead of silently wrapping.
