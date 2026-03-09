# E03ENTSTO-001: EntityKind Enum and EntityMeta Struct

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E01 (EntityId, Tick, core traits)

## Problem

E03 requires an explicit kind for every entity so invariants can reason about physicality and system rules. Before building the entity allocator or World struct, we need the `EntityKind` enum and `EntityMeta` struct.

## Assumption Reassessment (2026-03-09)

1. `worldwake-core` already exposes deterministic identity primitives in `crates/worldwake-core/src/ids.rs`, including `EntityId` with generation-based stale-id detection and `Tick` as the logical time type — confirmed.
2. There is currently no `entity.rs` module and no existing `EntityKind` or `EntityMeta` type in `worldwake-core` — confirmed.
3. The crate-level deterministic data policy already lives in `crates/worldwake-core/src/lib.rs`, and integration tests in `crates/worldwake-core/tests/policy.rs` already scan source files for `HashMap`, `HashSet`, `TypeId`, and `Box<dyn Any>` — confirmed.
4. This ticket cannot, by itself, enforce broad allocator/world invariants such as slot reuse or entity archival behavior; those belong to `E03ENTSTO-002` and later world-store tickets. Scope corrected accordingly.
5. The current crate test layout favors inline module tests for foundational types plus the existing integration policy suite. The original command `cargo test -p worldwake-core entity` is too loose to be a reliable targeted verification command for this crate. Scope corrected to target the new module tests directly.

## Architecture Check

1. A dedicated `entity.rs` module is cleaner than extending `ids.rs`: `ids.rs` should remain focused on reusable identifier/time newtypes, while entity classification and metadata are higher-level world-model concepts that future allocator/world code can import without mixing concerns.
2. `EntityMeta` should remain a pure data record in this ticket. Putting lifecycle helpers or allocator behavior on it now would blur ownership and make later `World` mutation surfaces harder to keep narrow and journal-friendly.
3. No backwards-compatibility shims or aliases are needed. These are new foundational types, so the cleanest architecture is to add them directly and let later tickets build on the canonical names.

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
- Crate-wide policy enforcement beyond continuing to satisfy the existing `tests/policy.rs` suite.

## Acceptance Criteria

### Tests That Must Pass

1. **Trait bounds**: `EntityKind` is `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned`.
2. **EntityMeta serialization**: bincode round-trip for `EntityMeta` with `archived_at: None` and `archived_at: Some(Tick(42))`.
3. **Deterministic ordering**: `EntityKind` variants sort deterministically (derived `Ord`).
4. **All variants roundtrip**: every `EntityKind` variant serializes and deserializes correctly via bincode.
5. **Module integration**: `lib.rs` exports the new entity module and re-exports `EntityKind` and `EntityMeta`.
6. **Existing suite**: `cargo test -p worldwake-core` continues green.

### Invariants

1. `EntityKind` and `EntityMeta` remain deterministic, serializable data only.
2. No floating-point types in `EntityKind` or `EntityMeta`.
3. New code continues to satisfy the existing crate policy tests that forbid `HashMap`, `HashSet`, `TypeId`, and `Box<dyn Any>` in authoritative source.

## Test Plan

### New/Modified Tests

In `crates/worldwake-core/src/entity.rs` (inline `#[cfg(test)]`):

- `entity_kind_trait_bounds` — compile-time assertion that `EntityKind` satisfies the deterministic serialization and ordering contract expected of authoritative enums.
- `entity_kind_all_variants_bincode_roundtrip` — proves every declared variant survives canonical serialization without lossy mapping or future alias pressure.
- `entity_meta_bincode_roundtrip_alive` — covers the live-entity metadata shape with `archived_at: None`.
- `entity_meta_bincode_roundtrip_archived` — covers the archived-entity metadata shape with `archived_at: Some(...)`.
- `entity_kind_deterministic_ordering` — verifies the declared variant set sorts deterministically after arbitrary input order, which is the behavior later `BTree*`-based world code depends on.

### Commands

```bash
cargo test -p worldwake-core entity_
cargo clippy --workspace && cargo test --workspace
```

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `crates/worldwake-core/src/entity.rs` with the canonical `EntityKind` enum and `EntityMeta` struct.
  - Registered `pub mod entity;` in `crates/worldwake-core/src/lib.rs` and re-exported `EntityKind` and `EntityMeta`.
  - Added focused inline tests for trait bounds, full enum round-trips, both metadata archive states, and deterministic ordering.
- Deviations from original plan:
  - Tightened the ticket before implementation so it no longer claimed allocator/world invariants that this slice cannot enforce yet.
  - Strengthened `EntityMeta` slightly beyond the original text by deriving `Eq` and `PartialEq`, which keeps it consistent with the existing test style for foundational value types and improves future comparison ergonomics without adding architectural complexity.
  - Narrowed the targeted verification command from the vague `cargo test -p worldwake-core entity` to `cargo test -p worldwake-core entity_`, which reliably selects the new module tests in the current suite layout.
- Verification results:
  - `cargo test -p worldwake-core entity_` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
