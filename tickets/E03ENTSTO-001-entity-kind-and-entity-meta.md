# E03ENTSTO-001: EntityKind Enum and EntityMeta Struct

**Status**: TODO
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E01 (EntityId, Tick, core traits)

## Problem

E03 requires an explicit kind for every entity so invariants can reason about physicality and system rules. Before building the entity allocator or World struct, we need the `EntityKind` enum and `EntityMeta` struct.

## What to Change

### 1. New module `entity.rs` in `worldwake-core/src/`

Define:

```rust
/// Classifies every entity for invariant checking and system routing.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum EntityKind {
    Agent,
    ItemLot,
    UniqueItem,
    Container,
    Facility,
    Place,
    Faction,
    Office,
    Contract,
    Rumor,
}
```

```rust
/// Authoritative metadata for a single entity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityMeta {
    pub kind: EntityKind,
    pub created_at: Tick,
    pub archived_at: Option<Tick>,
}
```

### 2. Register module in `lib.rs`

Add `pub mod entity;` and re-export `EntityKind` and `EntityMeta`.

## Files to Touch

- `crates/worldwake-core/src/entity.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Entity allocator logic (create_entity, archive_entity) — E03ENTSTO-002.
- ComponentTables, World struct — later tickets.
- `EventRecordProxy` variant — spec marks it optional, defer until event entities are actually materialized.
- Any gameplay logic or system rules that branch on EntityKind.

## Acceptance Criteria

### Tests That Must Pass

1. **Trait bounds**: `EntityKind` is `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned`.
2. **EntityMeta serialization**: bincode round-trip for `EntityMeta` with `archived_at: None` and `archived_at: Some(Tick(42))`.
3. **Deterministic ordering**: `EntityKind` variants sort deterministically (derived `Ord`).
4. **All variants roundtrip**: every `EntityKind` variant serializes and deserializes correctly via bincode.
5. **Existing suite**: `cargo test -p worldwake-core` continues green.

### Invariants

1. No `HashMap` or `HashSet` in any authoritative state.
2. No floating-point types in `EntityKind` or `EntityMeta`.
3. No `TypeId`, `Any`, or trait-object storage.

## Test Plan

### New Tests

In `crates/worldwake-core/src/entity.rs` (inline `#[cfg(test)]`):

- `entity_kind_trait_bounds` — compile-time assertion for required traits.
- `entity_kind_all_variants_bincode_roundtrip` — iterate all variants, serialize/deserialize each.
- `entity_meta_bincode_roundtrip_alive` — round-trip with `archived_at: None`.
- `entity_meta_bincode_roundtrip_archived` — round-trip with `archived_at: Some(...)`.
- `entity_kind_deterministic_ordering` — verify `Ord` produces a stable sort.

### Commands

```bash
cargo test -p worldwake-core entity
cargo clippy --workspace && cargo test --workspace
```
