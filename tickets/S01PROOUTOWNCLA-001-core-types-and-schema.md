# S01PROOUTOWNCLA-001: Define ProductionOutputOwnershipPolicy component and register in schema

**Status**: PENDING
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

## Architecture Check

1. Placing the types in `production.rs` alongside `WorkstationTag` keeps production-related types co-located
2. Registering on both `Facility` and `Place` matches `ResourceSource` scope, avoiding silent gaps for Place-based resource sources
3. No backward-compatibility aliases — new component, new enum, clean addition

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

## Files to Touch

- `crates/worldwake-core/src/production.rs` (modify — add types)
- `crates/worldwake-core/src/component_tables.rs` (modify — add macro entry)
- `crates/worldwake-core/src/component_schema.rs` (modify — register on Facility+Place)
- `crates/worldwake-core/src/delta.rs` (modify — add ComponentKind/ComponentValue variants)
- `crates/worldwake-core/src/lib.rs` (modify — re-export if needed)
- Any files with exhaustive matches on `ComponentKind` or `ComponentValue` (modify — add arms)

## Out of Scope

- Using the policy in harvest/craft handlers (S01PROOUTOWNCLA-004, -005)
- `create_item_lot_with_owner()` helper (S01PROOUTOWNCLA-002)
- Extending `can_exercise_control()` (S01PROOUTOWNCLA-003)
- Belief view changes (S01PROOUTOWNCLA-006)
- Pickup validation changes (S01PROOUTOWNCLA-007, -008)
- Test fixture migration (S01PROOUTOWNCLA-009)

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `ProductionOutputOwnershipPolicy` can be set and retrieved on a Facility entity
2. Unit test: `ProductionOutputOwnershipPolicy` can be set and retrieved on a Place entity
3. Unit test: `ProductionOutputOwnershipPolicy` cannot be set on entity kinds other than Facility/Place (if schema enforces this)
4. Unit test: All three `ProductionOutputOwner` variants round-trip through serde
5. Existing suite: `cargo test -p worldwake-core`
6. Full workspace: `cargo clippy --workspace` passes with no warnings

### Invariants

1. Component schema registration matches `ResourceSource` scope exactly (Facility + Place)
2. No default/fallback value for the policy — entities without it simply don't have one
3. `ComponentDelta` correctly captures before/after for `ProductionOutputOwnershipPolicy`
4. Deterministic ordering preserved (derives Ord, PartialOrd)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/production.rs` or test module — roundtrip, schema, set/get tests

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo build --workspace`
