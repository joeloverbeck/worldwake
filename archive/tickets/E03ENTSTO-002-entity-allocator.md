# E03ENTSTO-002: Generational Entity Allocator

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E03ENTSTO-001 (EntityKind, EntityMeta)

## Problem

The World needs a deterministic entity allocator that produces unique `EntityId` values with generational slot reuse detection. Archival must mark entities as non-live without destroying history. Slot reuse must increment generation to detect stale references.

## Reassessed Assumptions

- `EntityId` already exists in [`crates/worldwake-core/src/ids.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/ids.rs) with `slot` + `generation`, plus deterministic ordering and bincode tests.
- `EntityKind` and `EntityMeta` already exist in [`crates/worldwake-core/src/entity.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/entity.rs); this ticket must build on them rather than redefining entity lifecycle metadata.
- `WorldError` already exists in [`crates/worldwake-core/src/error.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/error.rs); allocator failures should use that type instead of introducing a ticket-local error.
- There is no allocator module yet in `worldwake-core`, so this ticket still needs to add one and re-export it from `lib.rs`.
- The original ticket proposed duplicating allocator truth across `entities`, `generations`, and `free_slots`. That is needlessly brittle. A single per-slot record plus a deterministic free-list is a cleaner authority boundary and a better fit for later `World` ownership.

## What to Change

### 1. New module `allocator.rs` in `worldwake-core/src/`

Define an `EntityAllocator` built around a single per-slot record:

```rust
/// Deterministic generational entity allocator.
///
/// Uses a deterministic free-list for slot reuse. Slot lifecycle state lives
/// in one map so generation and metadata cannot drift apart.
pub struct EntityAllocator {
    /// Per-slot lifecycle record. Index = slot number.
    slots: BTreeMap<u32, SlotRecord>,
    /// Free slots available for reuse, ordered for determinism.
    free_slots: BTreeSet<u32>,
    /// Next slot id if no free slots available.
    next_slot: u32,
}

struct SlotRecord {
    generation: u32,
    meta: Option<EntityMeta>,
}
```

`SlotRecord::meta` is `Some` for live or archived entities and `None` only after purge. This keeps archival history until explicit purge while preserving the generation needed for stale-id detection.

Public API:

- `EntityAllocator::new() -> Self`
- `create_entity(&mut self, kind: EntityKind, created_at: Tick) -> EntityId` — allocates a new or reused slot, sets metadata, returns fresh id.
- `archive_entity(&mut self, id: EntityId, tick: Tick) -> Result<(), WorldError>` — sets `archived_at`, does NOT free the slot yet or delete metadata.
- `purge_entity(&mut self, id: EntityId) -> Result<(), WorldError>` — requires an already-archived entity, clears metadata, increments the retained slot generation, and returns the slot to the free list.
- `is_alive(&self, id: EntityId) -> bool` — true if entity exists, generation matches, and not archived.
- `is_archived(&self, id: EntityId) -> bool` — true if entity exists, generation matches, and archived_at is Some.
- `get_meta(&self, id: EntityId) -> Option<&EntityMeta>` — returns metadata if generation matches.
- `entity_ids(&self) -> impl Iterator<Item = EntityId>` — all live (non-archived) entity ids in sorted order.

### 2. Register module in `lib.rs`

Add `pub mod allocator;` and re-export `EntityAllocator`.

## Files to Touch

- `crates/worldwake-core/src/allocator.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- ComponentTables or World struct — E03ENTSTO-003 and E03ENTSTO-006.
- Component CRUD — E03ENTSTO-004.
- Any event journaling or event emission on entity creation/archival — that's E06.
- Topology integration — already done in E02.
- Changing the existing `EntityId`, `EntityKind`, `EntityMeta`, or `WorldError` shapes unless allocator implementation exposes a real mismatch.

## Acceptance Criteria

### Tests That Must Pass

1. **Unique ids**: Creating N entities produces N distinct `EntityId` values.
2. **Archival marks non-live**: After `archive_entity`, `is_alive` returns false, `is_archived` returns true.
3. **Stale id detection**: After purge + reuse, old `EntityId` (lower generation) is not found via `get_meta`.
4. **Slot reuse increments generation**: After purge, next entity at same slot has generation + 1.
5. **Archive does not delete**: After archive, `get_meta` still returns the metadata.
6. **Purge requires archived**: Calling `purge_entity` on a live entity returns an error.
7. **Double archive errors**: Calling `archive_entity` on an already-archived entity returns an error.
8. **entity_ids() deterministic**: Iterator yields ids in slot-major sorted order.
9. **Serialization round-trip**: `EntityAllocator` serializes/deserializes via bincode, preserving all state.
10. **Existing suite**: `cargo test -p worldwake-core` continues green.

### Invariants

1. No `HashMap` or `HashSet` — all internal storage uses `BTreeMap`/`BTreeSet`.
2. Archival does not silently delete authoritative history.
3. Slot reuse always increments generation.
4. Iteration order is deterministic (sorted by slot).
5. Slot lifecycle state has a single source of truth; generation and metadata are not stored in separate authoritative maps.

## Test Plan

### New Tests

In `crates/worldwake-core/src/allocator.rs` (inline `#[cfg(test)]`):

- `create_produces_unique_ids`
- `archive_marks_non_live`
- `archived_entity_still_has_meta`
- `purge_frees_slot`
- `slot_reuse_increments_generation`
- `stale_id_not_found_after_reuse`
- `purge_live_entity_errors`
- `double_archive_errors`
- `entity_ids_sorted_order`
- `allocator_bincode_roundtrip`

### Commands

```bash
cargo test -p worldwake-core allocator
cargo clippy --workspace && cargo test --workspace
```

## Architectural Rationale

This ticket remains worth doing. The current architecture has `EntityId` semantics and lifecycle metadata, but no authoritative allocator yet. Adding one now is a net improvement because it establishes a deterministic ownership boundary before `World`, component tables, and queries depend on entity lifecycle.

The corrected design is better than the original proposal because:

- one `slots` map is easier to keep correct than separate `entities` and `generations` maps
- purge semantics become explicit: history stays until purge, then metadata clears while generation remains
- later `World` integration can treat the allocator as the sole authority for entity liveness instead of merging partial truths
- serialization is simpler and less error-prone because all slot state round-trips together

## Outcome

- Completion date: 2026-03-09
- Actual changes:
  - added `crates/worldwake-core/src/allocator.rs` with a deterministic generational `EntityAllocator`
  - re-exported `EntityAllocator` from `crates/worldwake-core/src/lib.rs`
  - added allocator unit coverage for unique ids, archive/purge lifecycle, stale-id invalidation, deterministic iteration, and bincode round-trip
- Deviations from original plan:
  - implemented a single `slots: BTreeMap<u32, SlotRecord>` authority instead of separate `entities` and `generations` maps
  - kept archive semantics as metadata-preserving and made purge the only operation that clears slot metadata and increments generation
- Verification:
  - `cargo test -p worldwake-core allocator`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
