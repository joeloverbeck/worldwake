# E10PROTRA-006: RecipeDefinition + RecipeRegistry in worldwake-sim

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new recipe system in sim crate
**Deps**: E10PROTRA-001 (RecipeId, WorkstationTag must exist in core)

## Problem

Production actions need data-driven recipe definitions analogous to `ActionDef`. Each recipe specifies inputs, outputs, work duration, workstation requirements, tool requirements, and body cost. The `RecipeRegistry` stores all available recipes and provides lookup by ID and by workstation tag. Without this, production logic would be hardcoded rather than data-driven.

## Assumption Reassessment (2026-03-10)

1. `ActionDef` / `ActionDefRegistry` pattern exists in `worldwake-sim` — confirmed. The recipe system follows the same registration pattern.
2. `RecipeId` and `WorkstationTag` will exist in `worldwake-core` after E10PROTRA-001.
3. `CommodityKind` and `Quantity` exist in `worldwake-core`.
4. `BodyCostPerTick` exists in `worldwake-core/src/needs.rs` — confirmed.
5. `NonZeroU32` is available from `std::num`.
6. The registry should be stored in `SimulationState` (spec requirement).
7. `BTreeMap` must be used for the registry's internal index (determinism invariant).

## Architecture Check

1. `RecipeDefinition` is a data struct, not a component — it lives in the registry, not on entities.
2. `RecipeRegistry` follows the same pattern as `ActionDefRegistry`: register definitions, look up by ID.
3. Additional query `recipes_for_workstation(tag)` enables workstation-specific recipe discovery.
4. No hidden loss/creation: if a recipe should produce scrap, it must appear in `outputs`.
5. Tool requirements are "possessed, not consumed" — tracked as `required_tool_kinds`.

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

Add `recipe_registry: RecipeRegistry` field to `SimulationState`.

### 4. Export from `lib.rs`

## Files to Touch

- `crates/worldwake-sim/src/recipe_def.rs` (new)
- `crates/worldwake-sim/src/recipe_registry.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add modules + re-exports)
- `crates/worldwake-sim/src/simulation_state.rs` (modify — add RecipeRegistry field)

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

### Invariants

1. `RecipeId` assignment is sequential and deterministic.
2. `by_workstation` uses `BTreeMap` (determinism).
3. No floating-point types.
4. Recipe conservation: the registry does not validate input/output balance — that is a design-time concern, not a runtime invariant. But no hidden creation or loss is possible because all inputs and outputs are explicit.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/recipe_def.rs` — construction, serialization, trait bounds
2. `crates/worldwake-sim/src/recipe_registry.rs` — registration, lookup by ID, lookup by workstation, iteration, edge cases

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
