# HARHYPENTIDE-002: Planning identity model — HypotheticalEntityId, PlanningEntityRef, unified maps

**Status**: ✅ COMPLETED
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
4. `PlannedStep.targets` in `crates/worldwake-ai/src/planner_ops.rs` is still `Vec<EntityId>` — confirmed. Typed planner targets remain future work in HARHYPENTIDE-004.
5. The authoritative transport path in `crates/worldwake-systems/src/transport_actions.rs` already performs exact split pickup in-world, but the planner transition in `crates/worldwake-ai/src/planner_ops.rs` still approximates by moving the full authoritative lot into possession — confirmed.
6. `CommitOutcome` and action-handler return-type hardening already exist in `worldwake-sim`/`worldwake-systems`; this ticket no longer owns that execution-layer work.
7. `PlanningState` derives `Clone` and all override maps use `BTreeMap`/`BTreeSet` (deterministic) — confirmed.

## Architecture Check

1. Introducing `PlanningEntityRef` as a sum type over `Authoritative(EntityId)` and `Hypothetical(HypotheticalEntityId)` is still the cleanest way to make planner identity honest without polluting the authoritative ECS.
2. This ticket should re-key only the `PlanningState` overlays that must represent hypothetical item lots and their relationships. Re-keying unrelated agent/resource overlays in the same change adds churn without current benefit.
3. Relationship overrides must be typed consistently. If keys become `PlanningEntityRef`, `direct_container_overrides` and `direct_possessor_overrides` values must also become `Option<PlanningEntityRef>`, not `Option<EntityId>`.
4. `BeliefView` remaining authoritative and `EntityId`-based is not a compatibility shim; it is the correct architectural boundary. Hypothetical queries belong on explicit planner-only methods.
5. This ticket is foundational. It improves architectural honesty and unblocks later exact-planning work, but it does not by itself deliver exact partial pickup or runtime rebinding.

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

Change only the hypothetical-entity-relevant overlays from `EntityId` to `PlanningEntityRef`:
- `entity_place_overrides: BTreeMap<PlanningEntityRef, Option<EntityId>>` (places are always authoritative)
- `direct_container_overrides: BTreeMap<PlanningEntityRef, Option<PlanningEntityRef>>`
- `direct_possessor_overrides: BTreeMap<PlanningEntityRef, Option<PlanningEntityRef>>`
- `commodity_quantity_overrides: BTreeMap<(PlanningEntityRef, CommodityKind), Quantity>`
- `removed_entities: BTreeSet<PlanningEntityRef>`
- `resource_quantity_overrides`, `reservation_shadows`, `needs_overrides`, and `pain_overrides` remain keyed by `EntityId` because this ticket does not introduce hypothetical resources or agents.

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
- `direct_container_ref(&self, entity: PlanningEntityRef) -> Option<PlanningEntityRef>`
- `move_entity_ref(self, entity: PlanningEntityRef, destination: EntityId) -> Self`
- `set_possessor_ref(self, entity: PlanningEntityRef, holder: PlanningEntityRef) -> Self`
- `set_container_ref(self, entity: PlanningEntityRef, container: PlanningEntityRef) -> Self`
- `set_quantity_ref(self, entity: PlanningEntityRef, commodity: CommodityKind, qty: Quantity) -> Self`
- `mark_removed_ref(self, entity: PlanningEntityRef) -> Self`
- `item_lot_commodity_ref(&self, entity: PlanningEntityRef) -> Option<CommodityKind>`

For `Authoritative` refs, fall back to snapshot. For `Hypothetical` refs, check overrides and registry only (no snapshot fallback).

### 7. Keep `BeliefView` authoritative-only

`BeliefView` remains `EntityId`-based. The existing `impl BeliefView for PlanningState` may delegate internally through `PlanningEntityRef::Authoritative(id)`, but this ticket must not pretend hypothetical entities are visible through `BeliefView`. Affordance queries and plan execution remain authoritative-only until HARHYPENTIDE-004/HARHYPENTIDE-005 migrate planner targets and runtime resolution.

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
- Any changes to `CommitOutcome` or action-handler return types; that work has already landed elsewhere

## Acceptance Criteria

### Tests That Must Pass

1. `PlanningState::spawn_hypothetical_lot` allocates deterministic IDs (0, 1, 2, ...).
2. `PlanningEntityRef::Authoritative` queries fall back to snapshot correctly.
3. `PlanningEntityRef::Hypothetical` queries return override data without snapshot fallback.
4. Hypothetical entity data (placement, possession, quantity) is queryable through unified `_ref` methods.
5. Cloning `PlanningState` preserves `next_hypothetical_id` counter correctly for search branching.
6. `mark_removed_ref` on hypothetical entities removes them from queries.
7. All existing `PlanningState` tests still pass (`BeliefView` remains authoritative-only).
8. Existing suite: `cargo test --workspace`
9. Existing lint: `cargo clippy --workspace`

### Invariants

1. `HypotheticalEntityId` is never treated as an `EntityId` — no implicit conversion exists.
2. Override maps use `BTreeMap` (deterministic iteration order).
3. `next_hypothetical_id` is monotonically increasing and part of cloned state.
4. `BeliefView` trait remains `EntityId`-based — not contaminated with planning types, and hypothetical entities do not leak through it.
5. Hypothetical entities always placed at authoritative places (no hypothetical places).
6. Hypothetical relationship edges use `PlanningEntityRef` on both ends; no mixed raw-`EntityId` relationship slots remain inside the planner identity model.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_state.rs` — new tests for `spawn_hypothetical_lot`, `PlanningEntityRef`-keyed queries, hypothetical relationship storage, and hypothetical entity lifecycle.
2. Existing `PlanningState` tests — updated only where needed to reflect the internal ref-based implementation while keeping `BeliefView` authoritative-only.

### Commands

1. `cargo test -p worldwake-ai planning_state`
2. `cargo test -p worldwake-ai planner_ops`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - Added planner-local `HypotheticalEntityId`, `PlanningEntityRef`, and `HypotheticalEntityMeta` in `worldwake-ai`.
  - Re-keyed the hypothetical-entity-relevant `PlanningState` overlays (`entity_place_overrides`, `direct_container_overrides`, `direct_possessor_overrides`, `commodity_quantity_overrides`, `removed_entities`) to use `PlanningEntityRef`.
  - Added deterministic hypothetical lot allocation plus planner-only `_ref` query/mutation methods for typed identity access.
  - Kept `BeliefView` authoritative-only and exported the new planner identity types from `worldwake-ai`.
  - Added regression coverage for deterministic ID allocation, authoritative fallback, hypothetical queryability, clone-safe branching, and non-leakage through `BeliefView`.
- Deviations from original plan:
  - Did not re-key unrelated overlays (`resource_quantity_overrides`, `reservation_shadows`, `needs_overrides`, `pain_overrides`) because hypothetical resources/agents remain out of scope and re-keying them now would add churn without architectural benefit.
  - Did not change `PlannedStep.targets`, runtime binding, carry-capacity belief data, or exact pickup planner semantics; those remain in their later tickets.
  - Did not touch `worldwake-sim` or `worldwake-systems`; the earlier ticket assumption about `CommitOutcome` ownership was stale because that work had already landed.
- Verification results:
  - `cargo test -p worldwake-ai planning_state` passed.
  - `cargo test -p worldwake-ai planner_ops` passed.
  - `cargo test -p worldwake-ai -- --nocapture` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
