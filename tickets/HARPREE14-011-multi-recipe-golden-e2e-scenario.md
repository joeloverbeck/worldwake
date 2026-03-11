# HARPREE14-011: Multi-recipe golden e2e scenario

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: HARPREE14-007 (benefits from named recipe lookup, soft dep)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-D01

## Problem

The golden e2e only tests a single recipe (Harvest Apples via orchard). Multi-recipe interactions (e.g., Harvest Grain -> Bake Bread, Chop Wood) are untested at the integration level. This is a coverage gap for the AI's ability to chain multi-step production plans.

## Assumption Reassessment (2026-03-11)

1. Golden e2e exists at `crates/worldwake-ai/tests/golden_e2e.rs` -- confirmed
2. Current scenario only registers and uses apple harvest -- needs verification during implementation
3. `RecipeRegistry` supports multiple recipe registration -- confirmed
4. `KnownRecipes` component exists for agents -- confirmed

## Architecture Check

1. A new test scenario (separate `#[test]` function) is the cleanest approach -- no risk of interfering with existing scenarios.
2. The new scenario should set up its own world state to avoid coupling with the existing scenario.

## What to Change

### 1. Add new test scenario to golden_e2e.rs

Create a new `#[test]` function (e.g., `test_multi_recipe_chain`) with:
- Multiple recipes registered: Harvest Apples (orchard), Harvest Grain (field), Bake Bread (mill, requires grain)
- Workstations placed at appropriate locations (field plot, mill)
- An agent with `KnownRecipes` including all recipes
- The agent positioned such that direct food (apples) is NOT available, forcing grain harvest + bread crafting

### 2. Assert recipe chaining behavior

Verify the agent:
- Harvests grain from the field
- Crafts bread at the mill using the harvested grain
- Consumes bread to satisfy hunger

### 3. Assert conservation invariants

Call `verify_live_lot_conservation()` and `verify_authoritative_conservation()` throughout the scenario.

### 4. Assert deterministic replay

Record state hashes, verify replay produces identical hashes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_e2e.rs` (modify -- add new test scenario)

## Out of Scope

- Modifying the existing golden e2e scenario
- Adding new recipe types to production code
- Changing `RecipeRegistry` or `RecipeDefinition` structure
- Modifying AI planning logic
- Changes to any production code files

## Acceptance Criteria

### Tests That Must Pass

1. New test: `test_multi_recipe_chain` -- agent chains harvest -> craft -> consume
2. Conservation invariants hold at every tick checkpoint
3. Deterministic replay produces identical hashes
4. All existing golden e2e scenarios pass unchanged
5. `cargo clippy --workspace` -- no new warnings

### Invariants

1. No production code changes
2. Existing golden e2e hashes unchanged
3. Conservation invariants (lot and authoritative) hold throughout new scenario
4. Deterministic replay verified

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_e2e.rs` -- new `test_multi_recipe_chain` function

### Commands

1. `cargo test -p worldwake-ai --test golden_e2e test_multi_recipe_chain` (targeted)
2. `cargo test -p worldwake-ai --test golden_e2e` (full e2e suite)
3. `cargo test --workspace`
4. `cargo clippy --workspace`
