# E10PROTRA-006: RecipeDefinition + RecipeRegistry in worldwake-sim

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new recipe system in sim crate
**Deps**: `archive/tickets/completed/E10PROTRA-001.md` (shared production schema in core)

## Problem

Production actions need data-driven recipe definitions analogous to `ActionDef`. Each recipe specifies inputs, outputs, work duration, workstation requirements, tool requirements, and body cost. The `RecipeRegistry` stores all available recipes and provides lookup by ID and by workstation tag. Without this, production logic would be hardcoded rather than data-driven.

## Assumption Reassessment (2026-03-10)

1. `RecipeId`, `WorkstationTag`, and `KnownRecipes` already exist in `crates/worldwake-core/src/production.rs` via completed E10 shared-schema work.
2. `CommodityKind`, `Quantity`, and `BodyCostPerTick` already exist in `worldwake-core` and satisfy the needed serialization and trait bounds.
3. `ActionDefRegistry` exists in `worldwake-sim`, but it only provides sequential-ID storage by `Vec<ActionDef>`. `RecipeRegistry` can mirror that deterministic base while adding a workstation secondary index.
4. `SimulationState` currently owns `world`, `event_log`, `scheduler`, `replay_state`, `controller_state`, and `rng_state`. Adding `RecipeRegistry` changes constructor shape, serialization, hashing, and save/load fixtures.
5. `save_load.rs`, `replay_execution.rs`, and `simulation_state.rs` contain direct `SimulationState::new(...)` call sites and tests that must be updated if the constructor becomes explicit about recipe state.
6. `NonZeroU32` is available from `std::num`.
7. Deterministic authoritative state must use `Vec` / `BTreeMap`, not hash-based collections.

## Architecture Check

1. `RecipeDefinition` is authoritative shared data, not a component. It belongs in sim-owned registry state, not on world entities.
2. `RecipeRegistry` should stay deterministic and small: sequential `RecipeId` assignment backed by `Vec<RecipeDefinition>` is acceptable here because recipes are authoritative registry entries, not free-floating entity data.
3. `recipes_for_workstation(tag)` should be a derived index maintained by the registry, not recomputed from ad hoc scans by later systems.
4. `SimulationState` should own the registry explicitly rather than hiding it behind globals or lazy static bootstrap. This keeps recipe availability in canonical saved state and replay hashing.
5. Tool requirements remain "possessed, not consumed" and should stay concrete via `required_tool_kinds`.
6. No conservation logic belongs in the registry itself. The registry defines recipes; later production actions enforce material movement against world state.

## What to Change

### 1. New module `crates/worldwake-sim/src/recipe_def.rs`

```rust
/// Data-driven production definition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecipeDefinition {
    pub name: String,
    pub inputs: Vec<(CommodityKind, Quantity)>,
    pub outputs: Vec<(CommodityKind, Quantity)>,
    pub work_ticks: NonZeroU32,
    pub required_workstation_tag: Option<WorkstationTag>,
    pub required_tool_kinds: Vec<CommodityKind>,
    pub body_cost_per_tick: BodyCostPerTick,
}
```

### 2. New module `crates/worldwake-sim/src/recipe_registry.rs`

```rust
/// Registry of all available recipes.
pub struct RecipeRegistry {
    recipes: Vec<RecipeDefinition>,
    by_workstation: BTreeMap<WorkstationTag, Vec<RecipeId>>,
}

impl RecipeRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, def: RecipeDefinition) -> RecipeId;
    pub fn get(&self, id: RecipeId) -> Option<&RecipeDefinition>;
    pub fn recipes_for_workstation(&self, tag: WorkstationTag) -> &[RecipeId];
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn iter(&self) -> impl Iterator<Item = (RecipeId, &RecipeDefinition)>;
}
```

### 3. Wire into `SimulationState`

Add `recipe_registry: RecipeRegistry` field to `SimulationState`, expose accessor methods, and make `SimulationState::new(...)` take the registry explicitly so authoritative roots are not partially implicit.

### 4. Export from `lib.rs`

## Files to Touch

- `crates/worldwake-sim/src/recipe_def.rs` (new)
- `crates/worldwake-sim/src/recipe_registry.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add modules + re-exports)
- `crates/worldwake-sim/src/simulation_state.rs` (modify — add RecipeRegistry field)
- `crates/worldwake-sim/src/save_load.rs` (modify — constructor/test fixture updates)
- `crates/worldwake-sim/src/replay_execution.rs` (modify — constructor/test fixture updates)

## Out of Scope

- Concrete recipe definitions for specific commodities (registered by action code in E10PROTRA-008/009)
- Harvest or Craft action logic
- ProductionJob creation or management
- AI recipe selection (E13)
- Recipe learning mechanics (future)

## Acceptance Criteria

### Tests That Must Pass

1. `RecipeDefinition` round-trips through bincode.
2. `RecipeRegistry::register` returns sequential `RecipeId` values starting from 0.
3. `RecipeRegistry::get(id)` returns the correct definition.
4. `RecipeRegistry::get(invalid_id)` returns `None`.
5. `RecipeRegistry::recipes_for_workstation(tag)` returns all recipes requiring that workstation.
6. `RecipeRegistry::recipes_for_workstation(unused_tag)` returns empty slice.
7. Recipes with `required_workstation_tag: None` do not appear in any workstation index.
8. `RecipeDefinition` with empty inputs (harvest-type) is valid.
9. `RecipeDefinition` with empty outputs is valid (waste disposal recipes).
10. Existing suite: `cargo test -p worldwake-sim`
11. `SimulationState` bincode/hash coverage includes `RecipeRegistry`.
12. Save/load round-trips preserve the recipe registry contents.

### Invariants

1. `RecipeId` assignment is sequential and deterministic.
2. `by_workstation` uses `BTreeMap` (determinism).
3. No floating-point types.
4. Recipe conservation: the registry does not validate input/output balance — that is a design-time concern, not a runtime invariant. But no hidden creation or loss is possible because all inputs and outputs are explicit.
5. `SimulationState` canonical state now includes recipe definitions; adding or mutating recipes changes serialized state and full-state hashing.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/recipe_def.rs` — construction, serialization, trait bounds
2. `crates/worldwake-sim/src/recipe_registry.rs` — registration, lookup by ID, lookup by workstation, iteration, edge cases
3. `crates/worldwake-sim/src/simulation_state.rs` — accessor, bincode, and hash coverage with non-empty registry
4. `crates/worldwake-sim/src/save_load.rs` — round-trip preserves recipe registry

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-10
- What actually changed:
  - Added `RecipeDefinition` in `crates/worldwake-sim/src/recipe_def.rs`.
  - Added deterministic `RecipeRegistry` in `crates/worldwake-sim/src/recipe_registry.rs` with sequential `RecipeId` assignment, lookup by ID, and workstation secondary indexing.
  - Re-exported both types from `crates/worldwake-sim/src/lib.rs`.
  - Added `recipe_registry: RecipeRegistry` to `SimulationState`, exposed immutable/mutable accessors, and made `SimulationState::new(...)` take the registry explicitly.
  - Included the recipe registry in simulation serialization, full-state hashing, replay bootstrap hashing, and save/load fixtures.
  - Added focused tests for recipe definition serialization, registry indexing/iteration behavior, simulation-state hash coverage, and save/load preservation.
- Deviations from original plan:
  - Corrected the ticket first to match the current codebase: `RecipeId`, `WorkstationTag`, and `KnownRecipes` already existed in core via completed E10 shared-schema work.
  - Expanded scoped touch points beyond `simulation_state.rs` because an explicit authoritative registry in `SimulationState` necessarily changes constructor call sites and save/replay coverage in `save_load.rs` and `replay_execution.rs`.
  - Kept `RecipeDefinition` free of an embedded `RecipeId`. The registry is the canonical authority that assigns deterministic IDs; this keeps recipe data plain while still making ID allocation explicit and testable.
  - Chose explicit `SimulationState::new(..., recipe_registry, ...)` ownership instead of an implicit empty/default registry. This is cleaner long-term because recipe availability is part of authoritative state rather than hidden bootstrap side data.
- Verification results:
  - `cargo test -p worldwake-sim` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `cargo test --workspace` ✅
