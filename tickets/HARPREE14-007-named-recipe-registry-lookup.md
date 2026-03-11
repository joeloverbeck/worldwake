# HARPREE14-007: Named recipe registry with string-keyed lookup

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes -- new method and index in RecipeRegistry
**Deps**: None (Wave 2, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-B01

## Problem

`RecipeRegistry` only supports positional `RecipeId(n)` lookup. Tests and setup code use fragile `RecipeId(0)`, `RecipeId(1)` references that break if registration order changes.

## Assumption Reassessment (2026-03-11)

1. `RecipeRegistry` has no `recipe_by_name` method -- confirmed
2. No `by_name` BTreeMap exists -- confirmed; only `by_workstation: BTreeMap<WorkstationTag, Vec<RecipeId>>`
3. `RecipeDefinition` has a `name` field (type `String`) -- needs verification during implementation
4. Methods: `new()`, `register()`, `get()`, `recipes_for_workstation()`, `len()`, `is_empty()`, `iter()` -- confirmed

## Architecture Check

1. A secondary `BTreeMap<String, RecipeId>` index built during `register()` is the standard approach for named lookup alongside positional access.
2. Duplicate name rejection prevents silent overwrites and ensures name uniqueness.
3. No backwards-compatibility shims. Pure additive API.

## What to Change

### 1. Add `by_name: BTreeMap<String, RecipeId>` field to `RecipeRegistry`

Secondary index mapping recipe names to their IDs.

### 2. Update `register()` to populate the name index

On each registration, insert the recipe name into `by_name`. Panic (matching existing style) or return an error if a duplicate name is registered.

### 3. Add `recipe_by_name(&self, name: &str) -> Option<(RecipeId, &RecipeDefinition)>`

Public method for name-based lookup.

### 4. Add tests

- Name-based lookup returns correct recipe
- Duplicate name registration panics/errors
- Empty registry returns None for any name lookup

## Files to Touch

- `crates/worldwake-sim/src/recipe_registry.rs` (modify)

## Out of Scope

- Changing `RecipeDefinition` structure
- Updating existing tests/code to USE name-based lookup (that can happen in HARPREE14-011)
- Modifying recipe registration in production code
- Changes to any other file

## Acceptance Criteria

### Tests That Must Pass

1. New test: `test_recipe_by_name_found` -- registered recipe found by name
2. New test: `test_recipe_by_name_not_found` -- unknown name returns None
3. New test: `test_duplicate_recipe_name_rejected` -- duplicate name panics/errors
4. New test: `test_empty_registry_name_lookup` -- empty registry returns None
5. All existing recipe registry tests pass unchanged
6. `cargo clippy --workspace` -- no new warnings

### Invariants

1. Existing `RecipeId`-based lookup unchanged
2. Registration order unchanged
3. No behavioral change to existing code
4. Golden e2e hashes identical

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/recipe_registry.rs` (test module) -- four new tests

### Commands

1. `cargo test -p worldwake-sim recipe` (targeted)
2. `cargo test --workspace`
3. `cargo clippy --workspace`
