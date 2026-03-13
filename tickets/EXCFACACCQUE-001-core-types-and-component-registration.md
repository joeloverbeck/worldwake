# EXCFACACCQUE-001 — Core Types, Component Registration, ActionDefId Relocation

**Spec sections**: §1, §2, §4 (queue_patience_ticks), Component Registration
**Crates**: `worldwake-core`, `worldwake-sim`

## Summary

Define the foundational types for exclusive facility access queues and register them as ECS components. Move `ActionDefId` from `worldwake-sim` to `worldwake-core` so that queue types (which are core components) can reference it without a circular dependency.

## Deliverables

### 1. Move `ActionDefId` to `worldwake-core`

Move the `ActionDefId` newtype from `crates/worldwake-sim/src/action_ids.rs` to a new or existing file in `crates/worldwake-core/src/ids.rs`. Re-export from `worldwake-sim` so that downstream code continues to compile unchanged.

- `ActionDefId` is currently defined via the `action_id_type!` macro in `worldwake-sim/src/action_ids.rs`.
- It must carry `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.
- `ActionHandlerId` and `ActionInstanceId` remain in `worldwake-sim`.

### 2. Define queue/policy types in `worldwake-core`

Create `crates/worldwake-core/src/facility_queue.rs`:

```rust
ExclusiveFacilityPolicy { grant_hold_ticks: NonZeroU32 }
FacilityUseQueue { next_ordinal: u32, waiting: BTreeMap<u32, QueuedFacilityUse>, granted: Option<GrantedFacilityUse> }
QueuedFacilityUse { actor: EntityId, intended_action: ActionDefId, queued_at: Tick }
GrantedFacilityUse { actor: EntityId, intended_action: ActionDefId, granted_at: Tick, expires_at: Tick }
```

All types derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

Add convenience methods on `FacilityUseQueue`:
- `enqueue(&mut self, actor, intended_action, tick) -> u32` — appends, returns ordinal
- `position_of(&self, actor) -> Option<u32>` — 0-indexed from head
- `has_actor(&self, actor) -> bool`
- `remove_actor(&mut self, actor) -> bool`
- `promote_head(&mut self, tick, grant_hold_ticks) -> Option<&GrantedFacilityUse>`
- `clear_grant(&mut self)`
- `grant_expired(&self, current_tick) -> bool`

### 3. Register components in `component_schema.rs`

Add two new component entries:
- `ExclusiveFacilityPolicy` on `EntityKind::Facility`
- `FacilityUseQueue` on `EntityKind::Facility`

Follow the existing tuple-block pattern in the `with_component_schema_entries!` macro.

### 4. Add `queue_patience_ticks` to agent profile

Add a `queue_patience_ticks: Option<NonZeroU32>` field to `UtilityProfile` (or the appropriate per-agent profile struct in `worldwake-core`). This governs how long an agent is willing to wait in a facility queue before replanning away. `None` means unlimited patience.

### 5. Export from `worldwake-core/src/lib.rs`

Public exports: `ExclusiveFacilityPolicy`, `FacilityUseQueue`, `QueuedFacilityUse`, `GrantedFacilityUse`, `ActionDefId`.

## Files to Touch

- `crates/worldwake-core/src/ids.rs` — add `ActionDefId`
- `crates/worldwake-core/src/facility_queue.rs` — **new file**, all queue/policy types
- `crates/worldwake-core/src/component_schema.rs` — register two components
- `crates/worldwake-core/src/component_tables.rs` — auto-expanded by macro (verify compilation)
- `crates/worldwake-core/src/utility_profile.rs` — add `queue_patience_ticks` field
- `crates/worldwake-core/src/lib.rs` — add `pub mod facility_queue;`, re-export types
- `crates/worldwake-sim/src/action_ids.rs` — remove `ActionDefId`, re-export from core
- `crates/worldwake-sim/src/lib.rs` — update re-export path for `ActionDefId`

## Out of Scope

- Action definitions or handlers (EXCFACACCQUE-002)
- The `facility_queue_system` (EXCFACACCQUE-003)
- Modifying harvest/craft preconditions (EXCFACACCQUE-004)
- Belief views, planning snapshot, planning state (EXCFACACCQUE-005, 006)
- AI planner ops, candidate generation, ranking (EXCFACACCQUE-007–010)
- Any behavioral logic — this ticket is pure type definitions and ECS registration

## Acceptance Criteria

### Tests that must pass
- `cargo test --workspace` — full workspace compiles and all existing tests pass (no regressions from ActionDefId move)
- Unit test: `FacilityUseQueue::enqueue` appends correctly and returns incrementing ordinals
- Unit test: `FacilityUseQueue::position_of` returns 0-indexed position from queue head
- Unit test: `FacilityUseQueue::has_actor` returns true only for queued actors
- Unit test: `FacilityUseQueue::remove_actor` removes and returns true; subsequent `has_actor` returns false
- Unit test: `FacilityUseQueue::promote_head` moves head to `granted` with correct `expires_at`
- Unit test: `FacilityUseQueue::promote_head` returns None on empty queue
- Unit test: `FacilityUseQueue::clear_grant` sets `granted` to None
- Unit test: `FacilityUseQueue::grant_expired` returns true when `current_tick >= expires_at`
- Unit test: an actor cannot appear twice in the same queue (enforced by `enqueue`)
- Unit test: `ExclusiveFacilityPolicy` and `FacilityUseQueue` can be inserted/retrieved via `ComponentTables` on a `Facility` entity
- Unit test: both types round-trip through serde (Serialize + Deserialize)

### Invariants that must remain true
- `ActionDefId` is usable from both `worldwake-core` and `worldwake-sim` without circular deps
- `BTreeMap` used for queue ordering (determinism)
- No `HashMap` or `HashSet` in any new types
- No floats in any new types
- All existing tests pass unmodified (ActionDefId move is purely structural)
- Component registration follows the existing macro pattern exactly
