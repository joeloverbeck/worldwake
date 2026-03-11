# HARPREE14-007: Named recipe registry with string-keyed lookup

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes -- new deterministic secondary index and duplicate-name invariant in `RecipeRegistry`
**Deps**: None (Wave 2, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-B01

## Problem

`RecipeRegistry` only supports positional `RecipeId(n)` lookup. That makes setup and test code fragile wherever recipe meaning is inferred from registration order instead of stable recipe identity.

## Assumption Reassessment (2026-03-11)

1. `RecipeRegistry` has no `recipe_by_name` method -- confirmed in `crates/worldwake-sim/src/recipe_registry.rs`.
2. No `by_name` index exists today -- confirmed; the only secondary index is `by_workstation: BTreeMap<WorkstationTag, Vec<RecipeId>>`.
3. `RecipeDefinition` already has `pub name: String` -- confirmed in `crates/worldwake-sim/src/recipe_def.rs`.
4. Existing `RecipeRegistry` API is `new()`, `register()`, `get()`, `recipes_for_workstation()`, `len()`, `is_empty()`, and `iter()` -- confirmed.
5. `RecipeRegistry` already has a 7-test unit module covering trait bounds, empty state, sequential IDs, workstation indexing, iteration order, and bincode roundtrip. This ticket must extend that test surface rather than treating it as absent.
6. Fragile positional recipe assumptions do exist in parts of the repo, but this ticket does not need to migrate those callers. Its job is to provide the robust primitive first.

## Architecture Check

1. The authoritative store should remain the existing `Vec<RecipeDefinition>` so `RecipeId` stays dense, deterministic, and serialization-compatible with current structure.
2. A secondary `BTreeMap<String, RecipeId>` built during `register()` is the cleanest way to add stable lookup without introducing aliasing or a second source of truth.
3. Duplicate-name rejection is architecturally beneficial, not incidental: if recipe names are used as stable identity for setup/tests, silent overwrite or last-write-wins behavior would create hidden ambiguity.
4. No backwards-compatibility shims or alias APIs. This is a direct improvement to the registry model, not a transitional layer.

## What to Change

### 1. Add `by_name: BTreeMap<String, RecipeId>` field to `RecipeRegistry`

Secondary index mapping canonical recipe names to their IDs. The `Vec<RecipeDefinition>` remains the source of truth for stored definitions.

### 2. Update `register()` to populate the name index

On each registration, insert the recipe name into `by_name`. Reject duplicate names immediately. For this ticket, panic-based rejection is acceptable because `register()` already returns `RecipeId` and the hardening goal is to prevent invalid registry state rather than introduce a wider API redesign.

### 3. Add `recipe_by_name(&self, name: &str) -> Option<(RecipeId, &RecipeDefinition)>`

Public method for name-based lookup.

### 4. Add tests

- Name-based lookup returns the registered ID and definition
- Unknown names return `None`
- Duplicate name registration panics
- Empty registry returns `None` for any name lookup
- Bincode roundtrip preserves named lookup behavior, not just structural equality

## Files to Touch

- `crates/worldwake-sim/src/recipe_registry.rs` (modify)

## Out of Scope

- Changing `RecipeDefinition` structure
- Migrating existing `RecipeId(0)`/`RecipeId(1)` call sites to named lookup
- Modifying production recipe registration flows outside `RecipeRegistry`
- Refactoring unrelated registry consumers

## Acceptance Criteria

### Tests That Must Pass

1. New test: named lookup returns the registered `RecipeId` and definition for a known recipe.
2. New test: unknown-name lookup returns `None`.
3. New test: duplicate recipe names are rejected via panic.
4. Existing empty-registry coverage is extended or supplemented so name lookup on an empty registry returns `None`.
5. Existing recipe registry tests continue to pass.
6. Registry serialization roundtrip still passes and named lookup works after deserialize.
7. `cargo test --workspace` passes.
8. `cargo clippy --workspace` passes with no new warnings.

### Invariants

1. Existing `RecipeId`-based lookup unchanged
2. Registration order unchanged
3. `Vec<RecipeDefinition>` remains the single authoritative recipe store
4. Duplicate recipe names cannot create ambiguous registry state
5. No caller-facing compatibility shim or alias path is introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/recipe_registry.rs` (test module) -- add named lookup and duplicate-name tests; extend serialization coverage to assert lookup semantics after roundtrip.

### Commands

1. `cargo test -p worldwake-sim recipe` (targeted)
2. `cargo test -p worldwake-sim recipe_registry`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Added a deterministic `by_name: BTreeMap<String, RecipeId>` secondary index to `RecipeRegistry`.
  - Added `recipe_by_name(&self, name: &str) -> Option<(RecipeId, &RecipeDefinition)>`.
  - Made `register()` reject duplicate recipe names before mutating registry state.
  - Expanded `recipe_registry.rs` tests to cover named lookup, duplicate rejection, and post-serialization lookup semantics.
- Deviations from original plan:
  - No cross-file caller migration was performed; that remains out of scope and matches the revised ticket scope.
  - Empty-registry and unknown-name lookup coverage were combined into one focused test rather than split into two separate tests.
- Verification results:
  - `cargo test -p worldwake-sim recipe_registry -- --nocapture` passed.
  - `cargo test -p worldwake-sim recipe -- --nocapture` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
