# E10PROTRA-002: ResourceSource component in worldwake-core

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new authoritative component registration
**Deps**: E10PROTRA-001 (CommodityKind already exists; no direct dep on 001 but logically follows)

## Problem

Harvesting must transfer goods from a concrete, depletable stock — not conjure them from a place tag. `ResourceSource` is the authoritative stored state that represents a finite supply of a commodity at a place or workstation, with optional regeneration. Without this, the production system violates Principle 3 (concrete state) and invariant 9.5 (conservation).

## Assumption Reassessment (2026-03-10)

1. No `ResourceSource` type exists in the codebase — confirmed.
2. `CommodityKind` exists in `items.rs`. `Quantity` exists in `numerics.rs`. Both are available for use.
3. The authoritative component registration pattern is well-established: `component_schema.rs` macro → `delta.rs` → generated APIs in `component_tables.rs` and `world.rs`.
4. `ResourceSource` should be registerable on `EntityKind::Facility` (workstation entities that are resource-bearing) and/or `EntityKind::Place`. The spec says "attached to a place or workstation." The kind predicate should accept both.
5. `NonZeroU32` is available from `std::num`.

## Architecture Check

1. `ResourceSource` is authoritative stored state per Principle 3 — "available workstations" and "can harvest" are derived read-models.
2. Placing it in `worldwake-core` allows both `worldwake-systems` (harvest action) and `worldwake-sim` (affordance queries) to read it.
3. Regeneration is stored as a rate parameter (`regeneration_ticks_per_unit`), not as a derived value. The actual regeneration tick logic belongs in a systems-crate system (E10PROTRA-007).
4. The component is on `Facility | Place` entities — the kind predicate must accept both.

## What to Change

### 1. Add `ResourceSource` to `crates/worldwake-core/src/production.rs`

```rust
/// Concrete depletable stock of a commodity at a place or workstation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceSource {
    pub commodity: CommodityKind,
    pub available_quantity: Quantity,
    pub max_quantity: Quantity,
    pub regeneration_ticks_per_unit: Option<NonZeroU32>,
}
impl Component for ResourceSource {}
```

### 2. Register in `component_schema.rs`

Add `ResourceSource` to the `with_authoritative_components!` macro with kind predicate `|kind| kind == EntityKind::Facility || kind == EntityKind::Place`.

### 3. Schema fanout

Update imports in `component_tables.rs`, `world.rs`, `delta.rs` as required by the schema-driven pattern.

## Files to Touch

- `crates/worldwake-core/src/production.rs` (modify — add ResourceSource)
- `crates/worldwake-core/src/component_schema.rs` (modify — add component registration)
- `crates/worldwake-core/src/lib.rs` (modify — re-exports if needed)
- `crates/worldwake-core/src/component_tables.rs` (modify — schema fanout imports/tests)
- `crates/worldwake-core/src/world.rs` (modify — generated API tests)
- `crates/worldwake-core/src/delta.rs` (modify — component inventory coverage)

## Out of Scope

- Regeneration tick logic (E10PROTRA-007)
- Harvest action (E10PROTRA-008)
- Any systems-crate changes
- Workstation creation helpers
- World-building / prototype world updates

## Acceptance Criteria

### Tests That Must Pass

1. `ResourceSource` can be inserted/retrieved/removed on `EntityKind::Facility` entities through the `World` API.
2. `ResourceSource` can be inserted/retrieved/removed on `EntityKind::Place` entities through the `World` API.
3. `ResourceSource` insertion is rejected for `EntityKind::Agent` and other non-matching kinds.
4. `ResourceSource` round-trips through bincode.
5. `ResourceSource` with `regeneration_ticks_per_unit: None` and `Some(NonZeroU32)` both serialize correctly.
6. `ComponentKind::ALL` and `ComponentValue` coverage include `ResourceSource`.
7. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `available_quantity` and `max_quantity` use `Quantity` — no raw integers, no floats.
2. Regeneration rate uses `Option<NonZeroU32>` — no zero-tick regeneration possible.
3. `ResourceSource` is authoritative stored state, not a derived value.
4. Component kind predicate restricts to `Facility | Place`.
5. No `HashMap`/`HashSet` used.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/production.rs` — construction, serialization, trait bounds
2. `crates/worldwake-core/src/component_tables.rs` — table CRUD for ResourceSource
3. `crates/worldwake-core/src/world.rs` — kind-restricted insertion/query + wrong-kind rejection
4. `crates/worldwake-core/src/delta.rs` — component inventory coverage

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
