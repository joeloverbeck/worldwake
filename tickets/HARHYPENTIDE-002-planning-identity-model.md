# HARHYPENTIDE-002: Planning identity model — HypotheticalEntityId, PlanningEntityRef, unified maps

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — planning state architecture (`worldwake-ai`)
**Deps**: None (can proceed in parallel with HARHYPENTIDE-001)
**Spec Reference**: HARDENING-hypothetical-entity-identity.md, Section A

## Problem

`PlanningState` can only overlay mutations on existing authoritative entities keyed by `EntityId`. It cannot create new hypothetical entities with stable identity, making it impossible for the planner to reason about entity-creating transitions (e.g. partial pickup producing a new lot).

## Assumption Reassessment (2026-03-12)

1. `PlanningState` in `crates/worldwake-ai/src/planning_state.rs:13` has all override maps keyed by `EntityId` — confirmed.
2. There is no concept of hypothetical entity identity anywhere in the codebase — confirmed.
3. `PlanningState` implements `BeliefView` for `EntityId`-based queries — confirmed.
4. Override maps include: `entity_place_overrides`, `direct_container_overrides`, `direct_possessor_overrides`, `resource_quantity_overrides`, `commodity_quantity_overrides`, `reservation_shadows`, `removed_entities`, `needs_overrides`, `pain_overrides` — confirmed.
5. `PlanningState` derives `Clone` and all override maps use `BTreeMap`/`BTreeSet` (deterministic) — confirmed.

## Architecture Check

1. Introducing `PlanningEntityRef` as a sum type over `Authoritative(EntityId)` and `Hypothetical(HypotheticalEntityId)` is the cleanest way to make identity honest without polluting the authoritative ECS.
2. Re-keying override maps from `EntityId` to `PlanningEntityRef` gives one code path per query. Hypothetical entities have no snapshot fallback — all data is in overrides.
3. No backward-compatibility aliases: the old `EntityId`-keyed API is replaced, not preserved alongside.

## What to Change

### 1. Introduce `HypotheticalEntityId`

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct HypotheticalEntityId(pub u32);
```

Deterministic, monotonically increasing within a search root. Not an ECS `EntityId`.

### 2. Introduce `PlanningEntityRef`

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum PlanningEntityRef {
    Authoritative(EntityId),
    Hypothetical(HypotheticalEntityId),
}
```

### 3. Introduce `HypotheticalEntityMeta`

```rust
pub struct HypotheticalEntityMeta {
    pub kind: EntityKind,
    pub item_lot_commodity: Option<CommodityKind>,
}
```

Minimal registry data — everything else (placement, quantity, possession) lives in override maps.

### 4. Re-key `PlanningState` override maps

Change all override map keys from `EntityId` to `PlanningEntityRef`:
- `entity_place_overrides: BTreeMap<PlanningEntityRef, Option<EntityId>>` (places are always authoritative)
- `direct_container_overrides: BTreeMap<PlanningEntityRef, Option<EntityId>>`
- `direct_possessor_overrides: BTreeMap<PlanningEntityRef, Option<EntityId>>`
- `commodity_quantity_overrides: BTreeMap<(PlanningEntityRef, CommodityKind), Quantity>`
- `removed_entities: BTreeSet<PlanningEntityRef>`
- etc.

Add new fields:
- `hypothetical_registry: BTreeMap<HypotheticalEntityId, HypotheticalEntityMeta>`
- `next_hypothetical_id: u32`

### 5. Add `spawn_hypothetical_lot` method

```rust
pub fn spawn_hypothetical_lot(&mut self, kind: EntityKind, commodity: CommodityKind) -> HypotheticalEntityId
```

Allocates from `next_hypothetical_id`, registers in `hypothetical_registry`, returns the new ID. Must be deterministic.

### 6. Add `PlanningEntityRef`-based query and mutation methods

New methods operating on `PlanningEntityRef`:
- `effective_place_ref(&self, entity: PlanningEntityRef) -> Option<EntityId>`
- `commodity_quantity_ref(&self, holder: PlanningEntityRef, kind: CommodityKind) -> Quantity`
- `direct_possessor_ref(&self, entity: PlanningEntityRef) -> Option<PlanningEntityRef>`
- `move_entity_ref(self, entity: PlanningEntityRef, destination: EntityId) -> Self`
- `set_possessor_ref(self, entity: PlanningEntityRef, holder: PlanningEntityRef) -> Self`
- `set_quantity_ref(self, entity: PlanningEntityRef, commodity: CommodityKind, qty: Quantity) -> Self`
- `mark_removed_ref(self, entity: PlanningEntityRef) -> Self`
- `item_lot_commodity_ref(&self, entity: PlanningEntityRef) -> Option<CommodityKind>`

For `Authoritative` refs, fall back to snapshot. For `Hypothetical` refs, check overrides and registry only (no snapshot fallback).

### 7. Preserve `BeliefView` impl for backward compatibility during migration

`BeliefView` remains `EntityId`-based. The existing `impl BeliefView for PlanningState` wraps `Authoritative(id)` internally when delegating to the new `PlanningEntityRef`-based methods. This preserves `BeliefView` consumers (affordance queries, etc.) while the planner's internal targeting migrates to `PlanningEntityRef`.

## Files to Touch

- `crates/worldwake-ai/src/planning_state.rs` (major modify — new types, re-keyed maps, new methods)
- `crates/worldwake-ai/src/lib.rs` (modify — export new types)

## Out of Scope

- `PlannedStep.targets` migration to `Vec<PlanningEntityRef>` (that is HARHYPENTIDE-004)
- Carry-capacity belief data (that is HARHYPENTIDE-003)
- Exact pickup transition rework (that is HARHYPENTIDE-006)
- Changes to `worldwake-sim` or `worldwake-systems`
- Changes to `worldwake-core`
- `MaterializationBindings` runtime table

## Acceptance Criteria

### Tests That Must Pass

1. `PlanningState::spawn_hypothetical_lot` allocates deterministic IDs (0, 1, 2, ...).
2. `PlanningEntityRef::Authoritative` queries fall back to snapshot correctly.
3. `PlanningEntityRef::Hypothetical` queries return override data without snapshot fallback.
4. Hypothetical entity data (placement, possession, quantity) is queryable through unified `_ref` methods.
5. Cloning `PlanningState` preserves `next_hypothetical_id` counter correctly for search branching.
6. `mark_removed_ref` on hypothetical entities removes them from queries.
7. All existing `PlanningState` tests still pass (backward-compatible `BeliefView` impl).
8. Existing suite: `cargo test --workspace`
9. Existing lint: `cargo clippy --workspace`

### Invariants

1. `HypotheticalEntityId` is never treated as an `EntityId` — no implicit conversion exists.
2. Override maps use `BTreeMap` (deterministic iteration order).
3. `next_hypothetical_id` is monotonically increasing and part of cloned state.
4. `BeliefView` trait remains `EntityId`-based — not contaminated with planning types.
5. Hypothetical entities always placed at authoritative places (no hypothetical places).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_state.rs` — new tests for `spawn_hypothetical_lot`, `PlanningEntityRef`-keyed queries, hypothetical entity lifecycle.
2. All existing `PlanningState` tests — updated to work with re-keyed maps (should pass through `BeliefView`).

### Commands

1. `cargo test -p worldwake-ai planning_state`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
