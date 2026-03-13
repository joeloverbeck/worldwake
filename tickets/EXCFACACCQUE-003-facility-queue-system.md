# EXCFACACCQUE-003 — `facility_queue_system` + System Manifest Registration

**Spec sections**: §4, §7, §8
**Crates**: `worldwake-systems`, `worldwake-sim`

## Summary

Add the `facility_queue_system` — a per-tick system that maintains facility queue state: prunes invalid entries, expires stale grants, and promotes the queue head to granted when eligible. Register it in the system manifest with correct execution order.

## Deliverables

### 1. `facility_queue_system` function

Create `facility_queue_system(world, event_log, current_tick)` in a new module `crates/worldwake-systems/src/facility_queue.rs`.

**Responsibilities (in this order):**

1. **Prune dead/departed/deallocated actors**: For each facility with a `FacilityUseQueue`, iterate waiting entries and remove any whose actor:
   - Is dead (`DeadAt` component exists)
   - Is no longer colocated with the facility (check placement relation)
   - No longer exists in the allocator

2. **Expire stale grants**: If `granted` is `Some` and `current_tick >= expires_at`, clear the grant.

3. **Prune permanently impossible queue heads** (with failure events): If the queue head's intended action is permanently impossible, prune it and emit a same-place `QueueHeadFailed` event. Permanent impossibility means:
   - Facility destroyed / deallocated
   - Facility lost the required `WorkstationTag`
   - Actor permanently incapable of the intended action

4. **Promote head to granted**: If `granted` is `None` and no matching exclusive action is currently running on this facility, promote the queue head to `granted` with `expires_at = current_tick + policy.grant_hold_ticks`.

**Critical**: Temporary stock depletion does NOT prune or skip the head. The queue simply stalls until stock regenerates or the agent replans away.

### 2. System manifest registration

Add a `FacilityQueue` variant to `SystemId` in `crates/worldwake-sim/src/system_manifest.rs`.

**Execution order**: After action commit/abort processing and before AI planning input generation. This ensures grants are promoted after operations complete and before agents make new decisions. Place it after `Combat` (the last domain system) and before any perception/AI systems.

### 3. System dispatch integration

Wire `facility_queue_system` into `crates/worldwake-sim/src/system_dispatch.rs` so it runs at the registered position.

### 4. Event tags

Add to `crates/worldwake-core/src/event_tag.rs`:
- `QueueGrantExpired` — grant forfeited due to timeout
- `QueueHeadFailed` — queue head pruned for permanent impossibility
- `QueueGrantPromoted` — head promoted to granted

## Files to Touch

- `crates/worldwake-systems/src/facility_queue.rs` — **new file**, system function
- `crates/worldwake-systems/src/lib.rs` — add `pub mod facility_queue;`
- `crates/worldwake-sim/src/system_manifest.rs` — add `FacilityQueue` system ID
- `crates/worldwake-sim/src/system_dispatch.rs` — wire dispatch for new system
- `crates/worldwake-sim/src/tick_step.rs` — ensure system runs in correct tick phase
- `crates/worldwake-core/src/event_tag.rs` — add event tags

## Out of Scope

- The `queue_for_facility_use` action (EXCFACACCQUE-002 — assumed complete)
- Grant requirement gate on harvest/craft (EXCFACACCQUE-004)
- Belief views or AI planning (EXCFACACCQUE-005–010)
- Conservation verification changes (queue state does not affect item conservation)
- Modifying existing systems (needs, production, trade, combat)

## Acceptance Criteria

### Tests that must pass
- Unit test: dead actor is pruned from queue
- Unit test: actor who departed the facility's place is pruned from queue
- Unit test: deallocated actor is pruned from queue
- Unit test: expired grant (`current_tick >= expires_at`) is cleared
- Unit test: queue head with permanently impossible action (workstation removed) is pruned and `QueueHeadFailed` event emitted
- Unit test: queue head with temporarily depleted stock is NOT pruned — queue stalls
- Unit test: when grant is None and queue is non-empty and no exclusive action running, head is promoted to granted
- Unit test: promoted grant has correct `expires_at = current_tick + grant_hold_ticks`
- Unit test: when an exclusive action IS running on the facility, no promotion occurs (wait for action to complete)
- Unit test: `QueueGrantPromoted` event is emitted on promotion
- Unit test: `QueueGrantExpired` event is emitted on grant expiry
- Unit test: system is idempotent — running twice in the same tick produces same result
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- At most one active grant per facility at any time
- Queue order is stable and deterministic for a fixed seed
- Pruning only occurs for permanent impossibility, never for temporary stock depletion
- The system does not start actions — it only advances queue state
- Events are emitted with `VisibilitySpec::SamePlace`
- System runs after action processing, before AI input generation
