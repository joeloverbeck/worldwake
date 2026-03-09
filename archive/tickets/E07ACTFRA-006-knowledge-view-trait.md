# E07ACTFRA-006: KnowledgeView Trait

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — defines the read-only knowledge abstraction used by action legality checks
**Deps**: E07ACTFRA-001 (IDs), E07ACTFRA-002 (semantic types), worldwake-core (`EntityId`, `EntityKind`, `CommodityKind`, `Quantity`, `TickRange`, etc.)
**Dependency Note**: Completed E07 prerequisites are archived under `archive/tickets/`. For this ticket, see [archive/tickets/E07ACTFRA-001-sim-crate-bootstrap-action-ids-action-status.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E07ACTFRA-001-sim-crate-bootstrap-action-ids-action-status.md) and [archive/tickets/E07ACTFRA-002-supporting-semantic-types.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E07ACTFRA-002-supporting-semantic-types.md).

## Problem

Affordance and start-gate legality checks must never depend directly on omniscient world access. The spec requires a read-only abstraction that can use authoritative state in Phase 1, then be replaced with a belief-backed implementation later without changing the action API. This is the type boundary that supports spec invariant 9.11 (world/belief separation).

## Assumption Reassessment (2026-03-09)

1. Spec 9.11 says: "Agents may react only to facts they perceived, inferred, remembered, or were told."
2. Spec E07 explicitly allows the Phase 1 implementation to read authoritative state through a swappable abstraction.
3. `Constraint` and `Precondition` already exist in `worldwake-sim` and define the actual Phase 1 query surface this trait must support:
   - `ActorAlive`
   - `ActorHasControl`
   - `ActorAtPlace(EntityId)`
   - `ActorHasCommodity { kind, min_qty }`
   - `ActorKind(EntityKind)`
   - `TargetExists(u8)`
   - `TargetAtActorPlace(u8)`
   - `TargetKind { .. }`
4. `worldwake-core` already exposes the underlying authoritative facts needed here, but not as a single affordance-oriented abstraction:
   - lifecycle: `World::is_alive`, `World::entity_kind`
   - placement: `World::effective_place`, `World::entities_effectively_at`
   - control source: `World::get_component_agent_data`
   - reservations: `World::reservations_for`
5. `worldwake-core` does **not** currently provide a canonical "entity has at least N of commodity K" helper. If affordance code needs that query, `WorldKnowledgeView` must implement it without changing `worldwake-core` in this ticket.
6. The belief system does not exist yet (E14), so this ticket only adds the Phase 1 authoritative adapter.

## Architecture Check

1. `KnowledgeView` should stay semantically narrow. It must expose the facts that action legality checks need, not a disguised `&World`.
2. Redundant convenience methods that can be derived from other semantic queries should be avoided. For example, `is_at_place(entity, place)` can be derived from `effective_place(entity) == Some(place)`.
3. Reservation checks should match the world model's interval-based reservations. A single-tick `is_reserved(entity, at_tick)` query is too narrow for an action framework whose reservation windows are `TickRange`-based.
4. Commodity checks should not leak inventory implementation details into affordance code. A quantity query is cleaner than forcing callers to reconstruct totals from raw lots.

## What to Change

### 1. Create `worldwake-sim/src/knowledge_view.rs`

Define a read-only trait aligned with current action semantics and reservation usage:

```rust
pub trait KnowledgeView {
    fn is_alive(&self, entity: EntityId) -> bool;
    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind>;
    fn effective_place(&self, entity: EntityId) -> Option<EntityId>;
    fn entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity;
    fn has_control(&self, entity: EntityId) -> bool;
    fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool;
}
```

Notes:
- `entities_at()` means entities effectively at a place, matching the current placement model.
- `commodity_quantity()` is the semantic inventory query for `ActorHasCommodity`.
- `reservation_conflicts()` checks overlap against a `TickRange`, not a single tick.

### 2. Create `worldwake-sim/src/world_knowledge_view.rs`

Implement `KnowledgeView` for a `WorldKnowledgeView<'w>` struct wrapping `&'w World`.

Implementation rules:
- delegate directly to authoritative `World` queries where they already exist
- treat `has_control()` as `ControlSource != None`
- implement `commodity_quantity()` by aggregating live `ItemLot` quantities that the holder currently possesses or contains via possessed containers
- keep the adapter read-only

### 3. Update `worldwake-sim/src/lib.rs`

Declare modules and re-export `KnowledgeView` plus `WorldKnowledgeView`.

## Files to Touch

- `crates/worldwake-sim/src/knowledge_view.rs` (new)
- `crates/worldwake-sim/src/world_knowledge_view.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- Belief-backed implementation (E14)
- Affordance query logic (E07ACTFRA-007)
- Start gate / action execution (E07ACTFRA-008+)
- Changes to `worldwake-core`
- New core inventory helper APIs

## Acceptance Criteria

### Tests That Must Pass

1. `WorldKnowledgeView` implements `KnowledgeView` (compile-time check)
2. `is_alive()` returns true for live entities and false for archived ones
3. `effective_place()` and `entities_at()` reflect effective placement, including contained entities
4. `commodity_quantity()` sums the holder's currently controlled matching lots deterministically
5. `entity_kind()` returns the correct kind for existing entities
6. `has_control()` returns true only for agents with `ControlSource::Human` or `ControlSource::Ai`
7. `reservation_conflicts()` returns true only when a live entity has an overlapping reservation in the queried range
8. Existing suite: `cargo test --workspace`

### Invariants

1. `KnowledgeView` is read-only: no `&mut self` methods
2. The trait does not expose `World` directly
3. Action legality code depends on `KnowledgeView`, never on `World` directly
4. The Phase 1 adapter can be replaced by a belief-backed implementation without changing action-query call sites

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/world_knowledge_view.rs`
   - compile-time trait check
   - live vs archived lifecycle query
   - effective placement queries including contained entities
   - commodity aggregation through possessed lots / possessed containers
   - control-source check
   - reservation overlap check

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

Implemented a narrow `KnowledgeView` trait plus `WorldKnowledgeView` in `worldwake-sim`, but adjusted the API from the original proposal in two important ways:

1. Replaced redundant `is_at_place()` / `location_of()` style queries with the semantic `effective_place()` query already used by the placement model.
2. Replaced single-tick reservation checks with `reservation_conflicts(entity, TickRange)`, matching the core reservation model's half-open interval semantics.

The final adapter also provides `commodity_quantity()` instead of only a boolean commodity check, so affordance and start-gate code can compare against semantic thresholds without reconstructing inventory totals themselves. In Phase 1 that quantity is derived from live possessed lots plus the contents of possessed containers, without changing `worldwake-core`.
