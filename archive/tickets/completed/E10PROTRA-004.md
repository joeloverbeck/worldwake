# E10PROTRA-004: KnownRecipes component in worldwake-core

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new authoritative component registration
**Deps**: `archive/tickets/completed/E10PROTRA-001.md` (completed shared-schema extraction for `RecipeId`)

## Problem

The spec requires recipe-gated production: agents cannot perform recipes they do not know. `KnownRecipes` is the per-agent authoritative component that tracks which recipes an agent has learned. Without this, "anyone at a forge can craft anything" — flattening agent diversity and collapsing role differentiation.

## Assumption Reassessment (2026-03-10)

1. `RecipeId` already exists in `crates/worldwake-core/src/production.rs` from `archive/tickets/completed/E10PROTRA-001.md` — confirmed.
2. No `KnownRecipes` type exists — confirmed.
3. The component should use `BTreeSet<RecipeId>` for deterministic ordering (project invariant: no `HashSet`).
4. Component goes on `EntityKind::Agent` only.
5. `crates/worldwake-core/src/component_schema.rs` is the single authoritative declaration point for ECS components, and its macro fanout affects `component_tables.rs`, `world.rs`, and `delta.rs`.
6. `crates/worldwake-core/src/component_schema.rs` also has a separate `with_txn_simple_set_components!` list; if `KnownRecipes` should participate in typed `WorldTxn` mutations and event-log deltas, this ticket must update that list and add `world_txn.rs` coverage.
7. Workspace verification currently includes a systems test that asserts the exact `ComponentKind::ALL` list; adding a new authoritative component will require updating that expectation too.

## Architecture Check

1. `KnownRecipes` is authoritative stored state — "which recipes can this agent perform right now" is a derived read-model (intersection of known recipes, available workstations, available inputs).
2. Using `BTreeSet<RecipeId>` ensures deterministic iteration and serialization.
3. Placing in `worldwake-core` allows the AI crate to query known recipes through `BeliefView` without importing `worldwake-systems`.
4. The current core architecture is macro-driven: new authoritative components should enter through the shared schema and reuse the existing typed world/txn/delta pipeline rather than introducing ad hoc storage or mutation paths.
5. `KnownRecipes` should remain recipe-granular, not capability-family based, until the codebase actually needs a higher-order abstraction. A capability alias layer here would add indirection without current benefit and would violate the repo's "no aliasing / no backward-compatibility layers" bias.

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

### 4. Transaction-layer component support

Add `KnownRecipes` to `with_txn_simple_set_components!` so later production systems can set and clear it through `WorldTxn` and produce typed `ComponentDelta`s without a side path.

### 5. Workspace-wide schema expectation

Update any existing exact `ComponentKind::ALL` expectations outside `worldwake-core` that intentionally mirror the authoritative schema inventory.

## Files to Touch

- `crates/worldwake-core/src/production.rs` (modify — add KnownRecipes)
- `crates/worldwake-core/src/component_schema.rs` (modify — add component registration)
- `crates/worldwake-core/src/lib.rs` (modify — re-exports)
- `crates/worldwake-core/src/component_tables.rs` (modify — schema fanout)
- `crates/worldwake-core/src/world.rs` (modify — generated API tests)
- `crates/worldwake-core/src/delta.rs` (modify — component inventory coverage)
- `crates/worldwake-core/src/world_txn.rs` (modify — transaction-layer setter/clearer coverage)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` (modify only if needed — authoritative schema expectation)

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
8. `WorldTxn::set_component_known_recipes(...)` records a typed `ComponentDelta::Set` and updates the world on commit.
9. `WorldTxn::clear_component_known_recipes(...)` records a typed `ComponentDelta::Removed` and updates the world on commit.
10. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Uses `BTreeSet`, not `HashSet` (determinism invariant).
2. Agent-only component.
3. Authoritative stored state — "performable recipes" is derived.
4. No floating-point types.
5. No capability aliases, shims, or backwards-compatibility layers around recipe knowledge.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/production.rs` — KnownRecipes construction, contains, serialization
2. `crates/worldwake-core/src/component_tables.rs` — table CRUD
3. `crates/worldwake-core/src/world.rs` — kind-restricted insertion + wrong-kind rejection
4. `crates/worldwake-core/src/delta.rs` — component inventory coverage
5. `crates/worldwake-core/src/world_txn.rs` — typed set/clear delta coverage
6. `crates/worldwake-systems/tests/e09_needs_integration.rs` — authoritative schema expectation, if impacted

### Commands

1. `cargo test -p worldwake-core known_recipes`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-10
- What actually changed:
  - Added `KnownRecipes` to `crates/worldwake-core/src/production.rs` as an authoritative agent component backed by `BTreeSet<RecipeId>`.
  - Added `KnownRecipes::new()` and `KnownRecipes::with(...)` for deterministic construction.
  - Registered `KnownRecipes` in the authoritative schema and re-exported it from `worldwake-core`.
  - Extended the macro fanout through `component_tables.rs`, `world.rs`, `delta.rs`, and `world_txn.rs`.
  - Added `WorldTxn` set/clear coverage so recipe knowledge participates in the standard typed component-delta pipeline.
  - Updated the exact `ComponentKind::ALL` expectation in `crates/worldwake-systems/tests/e09_needs_integration.rs` so workspace verification reflects the expanded authoritative schema.
- Deviations from original plan:
  - Corrected the ticket first to match the live codebase: `RecipeId` was already implemented and archived, and the real schema fanout also required `world_txn.rs` plus a downstream workspace test expectation.
  - Kept the design recipe-granular (`KnownRecipes`) instead of introducing a capability-family alias layer, because the current architecture does not need that indirection and the repo explicitly avoids alias/shim paths.
  - Expanded scope slightly to include transaction-layer support, which is architecturally cleaner than forcing later E10 tickets to add an ad hoc mutation path for recipe knowledge.
- Verification results:
  - `cargo test -p worldwake-core known_recipes` ✅
  - `cargo test -p worldwake-core` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `cargo test --workspace` ✅
- Outcome amended: 2026-03-10
- Post-completion refinement:
  - Removed the second hand-maintained transaction component list from `crates/worldwake-core/src/component_schema.rs`.
  - `WorldTxn` simple-component setter generation now derives from the same master component schema entries used by component tables, world APIs, and delta types.
  - This closes the drift point identified during implementation: future authoritative components no longer require separate schema and txn registration edits.
  - Re-verified with `cargo test -p worldwake-core known_recipes`, `cargo test -p worldwake-core`, `cargo test -p worldwake-systems authoritative_schema_includes_expected_shared_and_e09_components_and_fields`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.
