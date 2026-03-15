# S01PROOUTOWNCLA-001: Define ProductionOutputOwnershipPolicy component and register in schema

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — core component type, component_tables macro, component_schema, delta enums
**Deps**: None (foundational ticket for S01)

## Problem

Production output is currently materialized without ownership semantics. This ticket introduces the `ProductionOutputOwner` enum and `ProductionOutputOwnershipPolicy` component as the foundational types that all subsequent S01 tickets depend on.

## Assumption Reassessment (2026-03-15)

1. `WorkstationTag` and `ResourceSource` already exist in `crates/worldwake-core/src/production.rs` — confirmed
2. `ResourceSource` is registered on `EntityKind::Facility || EntityKind::Place` in `component_schema.rs` — confirmed
3. `ComponentValue` and `ComponentKind` enums exist in `crates/worldwake-core/src/delta.rs` — confirmed
4. `component_tables.rs` uses a macro pattern for typed storage — confirmed
5. No `ProductionOutputOwner` or `ProductionOutputOwnershipPolicy` exists yet — confirmed
6. Harvest and craft commits still create unowned ground lots via `WorldTxn::create_item_lot()` plus `set_ground_location()` in `crates/worldwake-systems/src/production_actions.rs` — confirmed
7. Ownership/control infrastructure already exists and is covered by tests in `crates/worldwake-core/src/world.rs`; this ticket should extend that substrate rather than introduce parallel ownership logic — confirmed
8. AI/runtime already flows pickup legality through `RuntimeBeliefView::can_control()` into planning snapshots (`controllable_by_actor`) and affordance filtering; that behavior belongs to later S01 tickets, not this foundational core-types ticket — confirmed
9. `worldwake-core` already has broad component-schema/component-delta regression coverage, so this ticket needs targeted tests for the new component rather than duplicate generic schema tests — confirmed

## Architecture Check

1. Placing the types in `production.rs` alongside `WorkstationTag` keeps production-related types co-located
2. Registering on both `Facility` and `Place` matches `ResourceSource` scope, avoiding silent gaps for Place-based resource sources
3. This ticket should stay strictly foundational: add the component, schema registration, storage, deltas, and exports only
4. No backward-compatibility aliases — new component, new enum, clean addition
5. Do not add defaults or implied fallback behavior here; later commit handlers must explicitly decide when to require the component

## What to Change

### 1. Add types to `crates/worldwake-core/src/production.rs`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ProductionOutputOwner {
    Actor,
    ProducerOwner,
    Unowned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ProductionOutputOwnershipPolicy {
    pub output_owner: ProductionOutputOwner,
}

impl Component for ProductionOutputOwnershipPolicy {}
```

### 2. Register in `component_tables.rs` macro

Add a `ProductionOutputOwnershipPolicy` entry to the macro-generated typed storage, following the existing pattern for other components.

### 3. Register in `component_schema.rs`

Register `ProductionOutputOwnershipPolicy` on `|kind| kind == EntityKind::Facility || kind == EntityKind::Place`, matching `ResourceSource` scope.

### 4. Add delta variants

Add `ProductionOutputOwnershipPolicy` variant to `ComponentKind` and `ComponentValue` enums in `delta.rs`, plus pattern-match arms wherever those enums are exhaustively matched.

### 5. Re-export from `crates/worldwake-core/src/lib.rs`

Ensure `ProductionOutputOwner` and `ProductionOutputOwnershipPolicy` are publicly accessible.

### 6. Add focused core tests

Add the smallest useful set of tests to prove:
- `ProductionOutputOwner` round-trips through serde and preserves deterministic ordering
- `ProductionOutputOwnershipPolicy` satisfies component bounds
- the component can be inserted on `Facility` and `Place`
- insertion is rejected on disallowed entity kinds
- transaction/component deltas can carry the new type

## Files to Touch

- `crates/worldwake-core/src/production.rs` (modify — add types)
- `crates/worldwake-core/src/component_tables.rs` (modify — add macro entry)
- `crates/worldwake-core/src/component_schema.rs` (modify — register on Facility+Place)
- `crates/worldwake-core/src/delta.rs` (modify — add ComponentKind/ComponentValue variants)
- `crates/worldwake-core/src/lib.rs` (modify — re-export if needed)
- Any files with exhaustive matches on `ComponentKind` or `ComponentValue` (modify — add arms)
- Existing core test modules that already own component/schema/delta coverage (modify — extend minimally)

## Out of Scope

- Using the policy in harvest/craft handlers (S01PROOUTOWNCLA-004, -005)
- `create_item_lot_with_owner()` helper (S01PROOUTOWNCLA-002)
- Extending `can_exercise_control()` (S01PROOUTOWNCLA-003)
- Belief view changes (S01PROOUTOWNCLA-006)
- Pickup validation changes (S01PROOUTOWNCLA-007, -008)
- Test fixture migration (S01PROOUTOWNCLA-009)
- Reworking planner/runtime affordance architecture; existing `can_control`/snapshot plumbing remains the integration path for later tickets

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `ProductionOutputOwnershipPolicy` can be set and retrieved on a Facility entity
2. Unit test: `ProductionOutputOwnershipPolicy` can be set and retrieved on a Place entity
3. Unit test: `ProductionOutputOwnershipPolicy` insertion is rejected on entity kinds other than Facility/Place
4. Unit test: All three `ProductionOutputOwner` variants round-trip through serde
5. Unit test: `ComponentValue::ProductionOutputOwnershipPolicy` reports the matching `ComponentKind`
6. Existing suite: `cargo test -p worldwake-core`
7. Full workspace: `cargo clippy --workspace` passes with no warnings

### Invariants

1. Component schema registration matches `ResourceSource` scope exactly (Facility + Place)
2. No default/fallback value for the policy — entities without it simply don't have one
3. `ComponentDelta` correctly captures before/after for `ProductionOutputOwnershipPolicy`
4. Deterministic ordering preserved (derives Ord, PartialOrd)
5. No new ownership or pickup logic is introduced in this ticket; it remains a pure substrate change

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/production.rs` — enum/component trait and serde coverage
2. `crates/worldwake-core/src/world.rs` or existing world component tests — Facility/Place registration and rejection on invalid kinds
3. `crates/worldwake-core/src/delta.rs` or existing delta tests — typed component-kind/value coverage

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo build --workspace`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Added `ProductionOutputOwner` and `ProductionOutputOwnershipPolicy` to `crates/worldwake-core/src/production.rs`
  - Registered `ProductionOutputOwnershipPolicy` in the authoritative component schema on `Facility` and `Place`
  - Exposed the new component through macro-generated component tables, world APIs, transaction setters, and typed delta enums
  - Re-exported the new types from `crates/worldwake-core/src/lib.rs`
  - Added targeted coverage in `production.rs`, `component_tables.rs`, `world.rs`, `delta.rs`, and `world_txn.rs`
- Deviations from original plan:
  - No manual edits were needed in `component_tables.rs` or world/transaction APIs beyond the schema-driven additions; the existing macro architecture generated most of the surface cleanly
  - Added an explicit `WorldTxn` delta test because transactional delta integrity is part of the real contract for later S01 tickets
- Verification results:
  - `cargo test -p worldwake-core` passed
  - `cargo build --workspace` passed
  - `cargo clippy --workspace` passed
