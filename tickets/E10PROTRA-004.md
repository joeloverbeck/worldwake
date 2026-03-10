# E10PROTRA-004: KnownRecipes component in worldwake-core

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new authoritative component registration
**Deps**: E10PROTRA-001 (RecipeId must exist)

## Problem

The spec requires recipe-gated production: agents cannot perform recipes they do not know. `KnownRecipes` is the per-agent authoritative component that tracks which recipes an agent has learned. Without this, "anyone at a forge can craft anything" — flattening agent diversity and collapsing role differentiation.

## Assumption Reassessment (2026-03-10)

1. `RecipeId` will exist in `production.rs` after E10PROTRA-001 — this ticket depends on that.
2. No `KnownRecipes` type exists — confirmed.
3. The component should use `BTreeSet<RecipeId>` for deterministic ordering (project invariant: no `HashSet`).
4. Component goes on `EntityKind::Agent` only.
5. The authoritative component registration pattern is stable and well-tested.

## Architecture Check

1. `KnownRecipes` is authoritative stored state — "which recipes can this agent perform right now" is a derived read-model (intersection of known recipes, available workstations, available inputs).
2. Using `BTreeSet<RecipeId>` ensures deterministic iteration and serialization.
3. Placing in `worldwake-core` allows the AI crate to query known recipes through `BeliefView` without importing `worldwake-systems`.

## What to Change

### 1. Add `KnownRecipes` to `crates/worldwake-core/src/production.rs`

```rust
/// Per-agent set of recipes this agent knows how to perform.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KnownRecipes {
    pub recipes: BTreeSet<RecipeId>,
}
impl Component for KnownRecipes {}
```

Provide `KnownRecipes::new()` (empty set) and `KnownRecipes::with(recipes: impl IntoIterator<Item = RecipeId>)`.

### 2. Register in `component_schema.rs`

Add `KnownRecipes` to the `with_authoritative_components!` macro, restricted to `EntityKind::Agent`.

### 3. Schema fanout + exports

Update `delta.rs`, `component_tables.rs`, `world.rs`, `lib.rs`.

## Files to Touch

- `crates/worldwake-core/src/production.rs` (modify — add KnownRecipes)
- `crates/worldwake-core/src/component_schema.rs` (modify — add component registration)
- `crates/worldwake-core/src/lib.rs` (modify — re-exports)
- `crates/worldwake-core/src/component_tables.rs` (modify — schema fanout)
- `crates/worldwake-core/src/world.rs` (modify — generated API tests)
- `crates/worldwake-core/src/delta.rs` (modify — component inventory coverage)

## Out of Scope

- `RecipeDefinition` or `RecipeRegistry` (E10PROTRA-006)
- Recipe learning mechanics (future epic)
- Harvest/Craft action logic (E10PROTRA-008, E10PROTRA-009)
- AI recipe selection (E13)

## Acceptance Criteria

### Tests That Must Pass

1. `KnownRecipes` can be inserted/retrieved/removed on Agent entities through the `World` API.
2. `KnownRecipes` insertion is rejected for non-Agent kinds.
3. `KnownRecipes` round-trips through bincode with multiple RecipeIds.
4. `KnownRecipes::new()` produces an empty set.
5. `KnownRecipes::with([RecipeId(0), RecipeId(2)])` contains exactly those IDs.
6. `KnownRecipes` iteration is deterministic (BTreeSet ordering).
7. `ComponentKind::ALL` and `ComponentValue` coverage include `KnownRecipes`.
8. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Uses `BTreeSet`, not `HashSet` (determinism invariant).
2. Agent-only component.
3. Authoritative stored state — "performable recipes" is derived.
4. No floating-point types.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/production.rs` — KnownRecipes construction, contains, serialization
2. `crates/worldwake-core/src/component_tables.rs` — table CRUD
3. `crates/worldwake-core/src/world.rs` — kind-restricted insertion + wrong-kind rejection
4. `crates/worldwake-core/src/delta.rs` — component inventory coverage

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
