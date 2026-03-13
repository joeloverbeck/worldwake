**Status**: COMPLETED

# EXCFACACCQUE-001 — Core Types, Component Registration, ActionDefId Relocation

**Spec sections**: §1, §2, §4 (queue_patience_ticks), Component Registration
**Crates**: `worldwake-core`, `worldwake-sim`

## Summary

Define the foundational types for exclusive facility access queues and register them as ECS components. Move `ActionDefId` from `worldwake-sim` to `worldwake-core` so queue types (which are authoritative core components) can reference it without introducing a circular dependency.

## Assumptions Reassessed

- `ActionDefId` still lives in `crates/worldwake-sim/src/action_ids.rs`; moving it remains necessary.
- The current codebase already distinguishes motive weighting from domain-specific behavior using dedicated profile components such as `TravelDispositionProfile` and `TradeDispositionProfile`.
- Because this repository does not want backwards-compatibility aliases, `worldwake-sim` should stop re-exporting `ActionDefId`. Callers must be updated to import `worldwake_core::ActionDefId` directly.
- `UtilityProfile` is still a decision-weight component consumed by ranking logic. Queue patience does not belong there; it should live in a queue-domain profile component instead.
- `component_schema.rs` remains the single source of truth for authoritative component registration. `ComponentTables`, `World`, `WorldTxn`, and delta plumbing are macro-derived from that schema, so no manual compatibility layer should be added elsewhere.

## Deliverables

### 1. Move `ActionDefId` to `worldwake-core`

Move the `ActionDefId` newtype from `crates/worldwake-sim/src/action_ids.rs` to `crates/worldwake-core/src/ids.rs`, preserving its stable representation and display format.

- `ActionDefId` is currently defined via the `action_id_type!` macro in `worldwake-sim/src/action_ids.rs`.
- It must carry `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.
- It must keep the stable `Display` prefix `adef`.
- `ActionHandlerId` and `ActionInstanceId` remain in `worldwake-sim`.
- Do not re-export `ActionDefId` from `worldwake-sim`. Update all affected code and tests to use `worldwake_core::ActionDefId`.

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
- `enqueue(&mut self, actor, intended_action, tick) -> Result<u32, FacilityQueueError>` — appends and returns ordinal
- `position_of(&self, actor) -> Option<u32>` — 0-indexed from head
- `has_actor(&self, actor) -> bool`
- `remove_actor(&mut self, actor) -> bool`
- `promote_head(&mut self, tick, grant_hold_ticks) -> Option<&GrantedFacilityUse>`
- `clear_grant(&mut self)`
- `grant_expired(&self, current_tick) -> bool`

`enqueue` must reject duplicate queue membership for the same actor via a typed error rather than silently inserting a second entry or panicking.

### 3. Define queue patience in a queue-domain disposition component

Create a per-agent authoritative component in `crates/worldwake-core/src/facility_queue.rs`:

```rust
FacilityQueueDispositionProfile { queue_patience_ticks: Option<NonZeroU32> }
```

Rationale:
- queue patience is a domain-specific waiting tolerance, not a utility weight
- this matches the existing architecture pattern used by `TravelDispositionProfile` and `TradeDispositionProfile`
- later AI tickets can read this profile without coupling queue semantics to motive ranking

`None` means unlimited patience.

### 4. Register components in `component_schema.rs`

Add three new component entries:
- `ExclusiveFacilityPolicy` on `EntityKind::Facility`
- `FacilityUseQueue` on `EntityKind::Facility`
- `FacilityQueueDispositionProfile` on `EntityKind::Agent`

Follow the existing tuple-block pattern in the `with_component_schema_entries!` macro.

### 5. Export from `worldwake-core/src/lib.rs`

Public exports: `ExclusiveFacilityPolicy`, `FacilityUseQueue`, `QueuedFacilityUse`, `GrantedFacilityUse`, `FacilityQueueDispositionProfile`, `ActionDefId`.

## Files to Touch

- `crates/worldwake-core/src/ids.rs` — add `ActionDefId`
- `crates/worldwake-core/src/facility_queue.rs` — **new file**, all queue/policy types
- `crates/worldwake-core/src/component_schema.rs` — register two components
- `crates/worldwake-core/src/component_tables.rs` — macro-derived surface only; add tests if needed, do not hand-maintain generated structure
- `crates/worldwake-core/src/lib.rs` — add `pub mod facility_queue;`, re-export types
- `crates/worldwake-core/src/test_utils.rs` — add fixture(s) for any new profile/component tests
- `crates/worldwake-sim/src/action_ids.rs` — remove `ActionDefId`
- `crates/worldwake-sim/src/lib.rs` — stop publicly re-exporting `ActionDefId`
- Affected `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, and `worldwake-cli` files/tests that currently import `worldwake_sim::ActionDefId`

## Out of Scope

- Action definitions or handlers (EXCFACACCQUE-002)
- The `facility_queue_system` (EXCFACACCQUE-003)
- Modifying harvest/craft preconditions (EXCFACACCQUE-004)
- Belief views, planning snapshot, planning state (EXCFACACCQUE-005, 006)
- AI planner ops, candidate generation, ranking (EXCFACACCQUE-007–010)
- Any queue runtime behavior — this ticket establishes types, storage, and imports only

## Acceptance Criteria

### Tests that must pass
- `cargo test -p worldwake-core`
- `cargo test -p worldwake-sim`
- `cargo test -p worldwake-ai`
- `cargo test -p worldwake-systems`
- `cargo test --workspace` — final full regression pass
- Unit test: `FacilityUseQueue::enqueue` appends correctly and returns incrementing ordinals
- Unit test: `FacilityUseQueue::enqueue` returns a duplicate-membership error for an already queued actor
- Unit test: `FacilityUseQueue::position_of` returns 0-indexed position from queue head
- Unit test: `FacilityUseQueue::has_actor` returns true only for queued actors
- Unit test: `FacilityUseQueue::remove_actor` removes and returns true; subsequent `has_actor` returns false
- Unit test: `FacilityUseQueue::promote_head` moves head to `granted` with correct `expires_at`
- Unit test: `FacilityUseQueue::promote_head` returns None on empty queue
- Unit test: `FacilityUseQueue::clear_grant` sets `granted` to None
- Unit test: `FacilityUseQueue::grant_expired` returns true when `current_tick >= expires_at`
- Unit test: an actor cannot appear twice in the same queue (enforced by `enqueue`)
- Unit test: `ExclusiveFacilityPolicy` and `FacilityUseQueue` can be inserted/retrieved via `ComponentTables` on a `Facility` entity
- Unit test: `FacilityQueueDispositionProfile` can be inserted/retrieved via `ComponentTables` on an `Agent` entity
- Unit test: both types round-trip through serde (Serialize + Deserialize)
- Unit test: `ActionDefId` round-trips through serde from `worldwake-core`
- Existing imports compile only through `worldwake_core::ActionDefId`; no `worldwake_sim::ActionDefId` alias remains

### Invariants that must remain true
- `ActionDefId` is defined exactly once in `worldwake-core` with no compatibility alias in `worldwake-sim`
- `BTreeMap` used for queue ordering (determinism)
- No `HashMap` or `HashSet` in any new types
- No floats in any new types
- Queue patience is stored in a queue-domain profile component, not in `UtilityProfile`
- Component registration follows the existing macro pattern exactly

## Outcome

- Completion date: 2026-03-13
- What actually changed:
  - Moved `ActionDefId` into `worldwake-core::ids` and updated workspace imports to use `worldwake_core::ActionDefId`.
  - Added `crates/worldwake-core/src/facility_queue.rs` with `ExclusiveFacilityPolicy`, `FacilityUseQueue`, `QueuedFacilityUse`, `GrantedFacilityUse`, `FacilityQueueDispositionProfile`, and `FacilityQueueError`.
  - Registered the new queue components/profile through `component_schema.rs` and added coverage in core component/delta tests.
- Deviations from original plan:
  - Did not keep a public `worldwake-sim` re-export for `ActionDefId`; the move was completed directly with caller fixes.
  - Did not add `queue_patience_ticks` to `UtilityProfile`; it now lives in `FacilityQueueDispositionProfile`, which matches the repo's domain-profile architecture.
  - Added a crate-private `#[cfg(test)]` import in `worldwake-sim` only to keep internal sim tests concise without restoring a public compatibility surface.
- Verification results:
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-cli`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
