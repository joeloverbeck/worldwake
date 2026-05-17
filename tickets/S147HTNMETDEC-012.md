# S147HTNMETDEC-012: Resolve recipe-input method preconditions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — updates HTN selector recipe-input template resolution and focused planner integration coverage.
**Deps**: `archive/tickets/S147HTNMETDEC-007.md` (`select_method` with actor-relative belief evaluation), `archive/tickets/S147HTNMETDEC-008.md` (planner integration in `build_stages`)

## Problem

S147 first-ship production methods such as `produce_with_gather` and `produce_with_purchase` use preconditions over `CommodityTemplate::RecipeInput { recipe: GoalRecipe, ordinal }`. During planner integration, the method dispatch path worked with concrete commodity templates, but canonical recipe-input methods did not select because `crates/worldwake-ai/src/htn/selector.rs` does not currently resolve `RecipeInput` templates when evaluating selector predicates.

Without this fix, ticket 008's method-first `build_stages` path is wired but canonical `ProduceCommodity` methods cannot match their resource-source or seller preconditions for recipe inputs.

## Assumption Reassessment (2026-05-17)

1. The shared boundary under audit is `htn::selector` template resolution for `BeliefPredicate::{ResourceSourceKnown, SellerKnown}` when the commodity template is `CommodityTemplate::RecipeInput`.
2. `build_stages` already receives `RecipeRegistry` and can expand selected method subgoals through planner-visible recipe definitions. The remaining gap is earlier: selector precondition evaluation cannot resolve recipe-input commodities before a method is selected.
3. The live `GoalKind` under test is `GoalKind::ProduceCommodity { recipe_id }`, using canonical method schemas from `build_method_registry`.
4. The selector must remain actor-relative and belief-only. Any recipe lookup added here must come from planner-visible recipe definitions or an explicitly threaded recipe registry, not from authoritative world state.
5. Invalid recipe ordinals must fail closed: a method with an out-of-range `RecipeInput` predicate must not match.

## Architecture Check

1. Resolve `RecipeInput` in the selector rather than weakening method preconditions or changing canonical method schemas. That preserves the S147 schema vocabulary and keeps method selection honest.
2. Thread only the recipe-definition surface needed by the selector, or reuse an existing planner-visible recipe access path if one is already available at implementation time.
3. No backwards-compatibility shims or duplicate method schemas are introduced.

## Verification Layers

1. Recipe-input selector predicate resolves to the first recipe input -> focused `htn::selector` unit test with `ResourceSourceKnown`.
2. Invalid recipe-input ordinal fails closed -> focused selector unit test.
3. Canonical `produce_with_gather` can now select for a `ProduceCommodity` goal with known resource source beliefs -> focused planner/selector integration test.
4. Existing planner dispatch remains stable -> `cargo test -p worldwake-ai --lib search::strategic`.

## What to Change

### 1. Add recipe-input resolution to selector preconditions

Update `crates/worldwake-ai/src/htn/selector.rs` so `CommodityTemplate::RecipeInput { recipe, ordinal }` can resolve against the active `ProduceCommodity` goal's recipe definition during `BeliefPredicate::ResourceSourceKnown` and `BeliefPredicate::SellerKnown` evaluation.

### 2. Thread recipe definitions into method selection if needed

If `select_method` does not already have access to recipe definitions, update the selector/planner boundary deliberately so recipe input resolution uses the same planner-visible `RecipeRegistry` used by `build_stages`.

### 3. Add focused coverage

Add focused tests for valid and invalid recipe-input predicates, plus a canonical `produce_with_gather` selection test that no longer needs a custom concrete-commodity method schema.

## Files to Touch

- `crates/worldwake-ai/src/htn/selector.rs` (modify)
- `crates/worldwake-ai/src/search/strategic.rs` (modify tests only if planner integration coverage is the best surface)

## Out of Scope

- Observer rendering of method choices (ticket 010).
- Method trace recording (ticket 009).
- Golden end-to-end scenarios (ticket 011).
- Adding new method schemas or new `PlannerOpKind` variants.

## Acceptance Criteria

### Tests That Must Pass

1. Focused selector test: `RecipeInput { recipe: GoalRecipe, ordinal: 0 }` resolves to the first recipe input for `ResourceSourceKnown`.
2. Focused selector test: invalid recipe input ordinal does not match.
3. Focused canonical method-selection test: `produce_with_gather` selects for a `ProduceCommodity` goal when the actor knows the first input's resource source.
4. Existing suite: `cargo test -p worldwake-ai --lib htn::selector`.
5. Existing suite: `cargo test -p worldwake-ai --lib search::strategic`.

### Invariants

1. Selector evaluation remains actor-relative and belief-only.
2. Missing recipe definitions or out-of-range ordinals fail closed.
3. Canonical method schemas remain the source of truth; tests do not rely on duplicate concrete-commodity substitute schemas.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/htn/selector.rs` — recipe-input predicate resolution and invalid ordinal tests.
2. `crates/worldwake-ai/src/search/strategic.rs` or selector integration tests — canonical `produce_with_gather` selection case.

### Commands

1. `cargo test -p worldwake-ai --lib htn::selector`
2. `cargo test -p worldwake-ai --lib search::strategic`
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
