# S147HTNMETDEC-012: Resolve recipe-input method preconditions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — updates HTN selector recipe-input template resolution and focused selector coverage.
**Deps**: `archive/tickets/S147HTNMETDEC-007.md` (`select_method` with actor-relative belief evaluation), `archive/tickets/S147HTNMETDEC-008.md` (planner integration in `build_stages`)

## Problem

S147 first-ship production methods such as `produce_with_gather` and `produce_with_purchase` use preconditions over `CommodityTemplate::RecipeInput { recipe: GoalRecipe, ordinal }`. During planner integration, the method dispatch path worked with concrete commodity templates, but canonical recipe-input methods did not select because `crates/worldwake-ai/src/htn/selector.rs` did not resolve `RecipeInput` templates when evaluating selector predicates.

Without this fix, ticket 008's method-first `build_stages` path was wired but canonical `ProduceCommodity` methods could not match their resource-source or seller preconditions for recipe inputs.

## Assumption Reassessment (2026-05-17)

1. The shared boundary under audit is `htn::selector` template resolution for `BeliefPredicate::{ResourceSourceKnown, SellerKnown}` and matching `LocationKnown(EntityCriterion::{ResourceSource, Seller})` checks when the commodity template is `CommodityTemplate::RecipeInput`.
2. `RuntimeBeliefView` already exposes `recipe_definition`, and the selector test belief view already carried recipe definitions. No new authoritative world read or parallel recipe source is needed.
3. The live `GoalKind` under test is `GoalKind::ProduceCommodity { recipe_id }`, using canonical method schemas from `build_method_registry`.
4. Missing recipe definitions and out-of-range input ordinals must fail closed rather than selecting a method from incomplete context.

## Architecture Check

1. `resolve_commodity` now resolves `RecipeInput` through `resolve_recipe(goal, recipe)` and `belief_view.recipe_definition(recipe_id)`, then indexes the recipe input by ordinal.
2. Existing selector predicate evaluation remains actor-relative and belief-only; it uses the same `RuntimeBeliefView` contract already used by `OwnsInputsForRecipe`.
3. Canonical method schemas remain unchanged. The positive canonical test now uses `build_method_registry()` and selects method id `5` (`produce_with_gather`) from recipe-input resource-source beliefs.
4. No backwards-compatibility shims or duplicate substitute schemas were added.

## Verified Layers

1. Recipe-input selector predicate resolves to the first recipe input -> `recipe_input_resource_source_precondition_resolves_first_recipe_input`.
2. Invalid recipe-input ordinal fails closed -> `recipe_input_resource_source_precondition_fails_closed_for_invalid_ordinal`.
3. Canonical `produce_with_gather` can select for a `ProduceCommodity` goal with known resource source beliefs -> `canonical_produce_with_gather_selects_from_known_recipe_input_source`.
4. Existing planner dispatch remains stable -> `cargo test -p worldwake-ai --lib search::strategic`.

## Landed Changes

### 1. Recipe-input commodity resolution

`crates/worldwake-ai/src/htn/selector.rs::resolve_commodity` now accepts the `RuntimeBeliefView` and resolves `CommodityTemplate::RecipeInput { recipe, ordinal }` by reading the planner-visible recipe definition for the goal recipe.

### 2. Predicate call sites

`ResourceSourceKnown`, `SellerKnown`, `OwnedCommodityBelowThreshold`, and the matching `LocationKnown` resource-source/seller checks now pass the belief view into `resolve_commodity`, so all selector commodity-template predicates share the same recipe-input behavior.

### 3. Focused tests

Added selector tests for valid recipe-input resolution, invalid ordinal failure, and canonical `produce_with_gather` selection from `build_method_registry()`.

## Landed Files

- `crates/worldwake-ai/src/htn/selector.rs` (recipe-input resolution and tests)

## Outcome

Completed: 2026-05-17.

Canonical S147 production methods can now satisfy resource-source method preconditions against recipe inputs through planner-visible recipe definitions. Invalid recipe input ordinals remain non-matching.

## Deviations

- The fix stayed entirely in `htn::selector`; no planner signature change was needed because `RuntimeBeliefView::recipe_definition` was already available at the selector boundary.

## Out of Scope

- Observer rendering of method choices (ticket 010).
- Method trace recording (ticket 009).
- Golden end-to-end scenarios (ticket 011).
- Adding new method schemas or new `PlannerOpKind` variants.

## Acceptance Result

### Tests Passed

1. `cargo test -p worldwake-ai --lib htn::selector`
2. `cargo test -p worldwake-ai --lib search::strategic`
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

### Invariants

1. Selector evaluation remains actor-relative and belief-only.
2. Missing recipe definitions or out-of-range ordinals fail closed.
3. Canonical method schemas remain the source of truth.

## Verification Result

Passed the selector focused suite, the strategic search regression suite, and all-target clippy for `worldwake-ai`.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/htn/selector.rs` — added recipe-input predicate resolution, invalid ordinal, and canonical `produce_with_gather` selection tests.

### Verification Commands

1. `cargo test -p worldwake-ai --lib htn::selector`
2. `cargo test -p worldwake-ai --lib search::strategic`
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
