# E07ACTFRA-006: KnowledgeView Trait

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — defines the affordance evaluation abstraction
**Deps**: E07ACTFRA-001 (IDs), worldwake-core (EntityId, EntityKind, CommodityKind, etc.)

## Problem

Affordance queries must never depend directly on omniscient world access. The spec requires an abstraction (`KnowledgeView`) that can be backed by authoritative state in Phase 1 but swapped for a belief-backed view in later phases without changing the action API. This is the key abstraction that enforces spec invariant 9.11 (world/belief separation) at the type level.

## Assumption Reassessment (2026-03-09)

1. Spec 9.11: "Agents may react only to facts they perceived, inferred, remembered, or were told."
2. Phase 1 may back the view with authoritative state (spec says so explicitly).
3. `EntityId`, `EntityKind`, `CommodityKind`, `Quantity`, `ControlSource` all exist in worldwake-core.
4. The belief system does not exist yet (E14) — so Phase 1 uses a "world view" adapter.

## Architecture Check

1. `KnowledgeView` is a trait, not a concrete struct. This allows: (a) `WorldKnowledgeView` backed by `&World` for Phase 1, and (b) `BeliefKnowledgeView` backed by agent beliefs in Phase 3+.
2. The trait exposes only the queries that affordance evaluation needs — it is deliberately narrow to prevent accidental omniscience leaks.
3. No `&mut` methods — knowledge views are read-only.

## What to Change

### 1. Create `worldwake-sim/src/knowledge_view.rs`

Define the `KnowledgeView` trait:
```rust
pub trait KnowledgeView {
    fn is_alive(&self, entity: EntityId) -> bool;
    fn is_at_place(&self, entity: EntityId, place: EntityId) -> bool;
    fn location_of(&self, entity: EntityId) -> Option<EntityId>;
    fn entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn has_commodity(&self, entity: EntityId, kind: CommodityKind, min_qty: Quantity) -> bool;
    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind>;
    fn has_control(&self, entity: EntityId) -> bool;
    fn is_reserved(&self, entity: EntityId, at_tick: Tick) -> bool;
}
```

### 2. Create `worldwake-sim/src/world_knowledge_view.rs`

Implement `KnowledgeView` for a `WorldKnowledgeView<'w>` struct that wraps `&'w World`:
- Each method delegates to the authoritative world state
- This is the Phase 1 implementation; Phase 3 will add a belief-backed implementation

### 3. Update `worldwake-sim/src/lib.rs`

Declare modules, re-export trait and Phase 1 implementation.

## Files to Touch

- `crates/worldwake-sim/src/knowledge_view.rs` (new)
- `crates/worldwake-sim/src/world_knowledge_view.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- Belief-backed implementation (E14)
- Affordance query logic (E07ACTFRA-007)
- Constraint/precondition evaluation (E07ACTFRA-007/008)
- Changes to `World` or `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. `WorldKnowledgeView` implements `KnowledgeView` (compile-time check)
2. `is_alive()` returns true for living entities and false for archived ones
3. `is_at_place()` correctly checks entity location
4. `entities_at()` returns all entities at a given place
5. `has_commodity()` correctly checks inventory quantities
6. `entity_kind()` returns the correct kind for existing entities
7. `has_control()` returns true for entities with `ControlSource::Human` or `ControlSource::Ai`
8. `is_reserved()` correctly checks reservation state at a given tick
9. Existing suite: `cargo test --workspace`

### Invariants

1. `KnowledgeView` is a read-only trait — no `&mut self` methods
2. The trait does not expose the full `World` — only the queries affordance evaluation needs
3. Affordance code depends on `KnowledgeView`, never on `World` directly

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/world_knowledge_view.rs` — tests using `test_utils` to set up a world, then querying through the view

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`
