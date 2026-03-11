# HARPREE14-003: Prototype world entity accessor API

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes -- new public struct and function in topology
**Deps**: None (Wave 1, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-B03

## Problem

The golden e2e test (line 47) duplicates the private `prototype_entity()` function from `topology.rs` because there's no public way to get named entity IDs for the prototype world. Other test files that use `build_prototype_world()` face the same issue, leading to fragile, duplicated index arithmetic.

## Assumption Reassessment (2026-03-11)

1. `prototype_entity()` exists at line 559 of `topology.rs` and is private -- confirmed
2. Golden e2e defines its own `prototype_entity()` at line 47 as a `const fn` -- confirmed
3. No `PrototypeWorldEntities` struct exists in the codebase -- confirmed

## Architecture Check

1. A named struct with explicit fields is more robust than positional `prototype_entity(n)` calls -- field names document intent and won't break if registration order changes.
2. No backwards-compatibility shims. The private `prototype_entity()` can remain as an internal helper or be made public.

## What to Change

### 1. Add `PrototypeWorldEntities` struct to topology.rs

```rust
pub struct PrototypeWorldEntities {
    pub village_square: EntityId,
    pub orchard_farm: EntityId,
    pub general_store: EntityId,
    pub common_house: EntityId,
    pub rulers_hall: EntityId,
    pub guard_post: EntityId,
    pub public_latrine: EntityId,
    pub north_crossroads: EntityId,
    pub forest_path: EntityId,
    pub bandit_camp: EntityId,
    pub south_gate: EntityId,
    pub east_field_trail: EntityId,
}
```

### 2. Add `pub fn prototype_world_entities() -> PrototypeWorldEntities`

Returns the entity IDs matching the allocation order in `build_prototype_world()`. Can use the existing `prototype_entity()` internally.

### 3. Make `prototype_entity()` public or keep private

If kept private, `prototype_world_entities()` wraps it. Either way, the public API is the struct.

### 4. Update golden e2e to use `prototype_world_entities()`

Remove the local `prototype_entity()` const fn and manual slot constants. Use the struct fields instead.

## Files to Touch

- `crates/worldwake-core/src/topology.rs` (modify -- add struct + function)
- `crates/worldwake-ai/tests/golden_e2e.rs` (modify -- use new API, remove local helper)

## Out of Scope

- Changing `build_prototype_world()` behavior or place definitions
- Adding new places to the prototype world
- Modifying entity allocation order
- Changing any other test files (they can adopt the new API in separate tickets)

## Acceptance Criteria

### Tests That Must Pass

1. Golden e2e passes with identical state hashes
2. `cargo test -p worldwake-core topology` -- all existing topology tests pass
3. `cargo test --workspace` -- full suite passes
4. `cargo clippy --workspace` -- no new warnings

### Invariants

1. `build_prototype_world()` behavior unchanged
2. Entity allocation order unchanged
3. Golden e2e state hashes identical
4. No behavioral change of any kind

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_e2e.rs` -- modified to use `prototype_world_entities()` instead of local helper

### Commands

1. `cargo test -p worldwake-ai --test golden_e2e` (targeted)
2. `cargo test --workspace`
3. `cargo clippy --workspace`
