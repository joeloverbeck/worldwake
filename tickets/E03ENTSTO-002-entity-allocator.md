# E03ENTSTO-002: Generational Entity Allocator

**Status**: TODO
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E03ENTSTO-001 (EntityKind, EntityMeta)

## Problem

The World needs a deterministic entity allocator that produces unique `EntityId` values with generational slot reuse detection. Archival must mark entities as non-live without destroying history. Slot reuse must increment generation to detect stale references.

## What to Change

### 1. New module `allocator.rs` in `worldwake-core/src/`

Define an `EntityAllocator` struct:

```rust
/// Deterministic generational entity allocator.
///
/// Uses a free-list for slot reuse. Every reuse increments the slot's
/// generation so stale `EntityId` references compare unequal to fresh ones.
pub struct EntityAllocator {
    /// Per-slot metadata. Index = slot number.
    entities: BTreeMap<u32, EntityMeta>,
    /// Current generation per slot (even for freed slots).
    generations: BTreeMap<u32, u32>,
    /// Free slots available for reuse, ordered for determinism.
    free_slots: BTreeSet<u32>,
    /// Next slot id if no free slots available.
    next_slot: u32,
}
```

Public API:

- `EntityAllocator::new() -> Self`
- `create_entity(&mut self, kind: EntityKind, created_at: Tick) -> EntityId` — allocates a new or reused slot, sets metadata, returns fresh id.
- `archive_entity(&mut self, id: EntityId, tick: Tick) -> Result<(), WorldError>` — sets `archived_at`, does NOT free the slot yet or delete metadata.
- `purge_entity(&mut self, id: EntityId) -> Result<(), WorldError>` — removes metadata and returns slot to free list with incremented generation. Only valid on already-archived entities.
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
