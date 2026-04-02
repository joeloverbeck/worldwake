# S06COMOPPVAL-004: Recipe opportunity propagation (indirect value)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-sim` (commodity_opportunity.rs — indirect recipe value propagation)
**Deps**: S06COMOPPVAL-003

## Problem

The shared `commodity_opportunity_score` from ticket 003 returns `indirect_recipe_score: 0`. Without indirect recipe value, firewood has zero value to a hungry baker even though firewood + grain → bread (hunger relief). This ticket implements the bounded recipe-closure analysis that propagates value backward from recipe outputs to inputs.

## Assumption Reassessment (2026-04-02)

1. `RecipeRegistry` at `crates/worldwake-sim/src/recipe_registry.rs` stores `Vec<RecipeDefinition>` indexed by `RecipeId`, name, and `WorkstationTag`. Iteration via `.iter()` is deterministic (Vec order = registration order).
2. `RecipeDefinition` at `crates/worldwake-sim/src/recipe_def.rs` has `inputs: Vec<(CommodityKind, Quantity)>`, `outputs: Vec<(CommodityKind, Quantity)>`, `required_tool_kinds: Vec<UniqueItemKind>`, and workstation tag.
3. `KnownRecipes` at `crates/worldwake-core/src/production.rs` wraps `BTreeSet<RecipeId>`. Available via `GoalBeliefView` (need to verify exact method name — likely `known_recipes` or similar).
4. `CommodityValuationProfile` from ticket 001 provides `recipe_opportunity_depth: NonZeroU8`, `recipe_place_horizon: u8`, `indirect_value_decay_per_step: Permille`.
5. Workstation reachability requires checking believed place graph within `recipe_place_horizon` hops. `GoalBeliefView` exposes place/topology methods for this.
6. Multi-input recipes: if bread requires grain + firewood, firewood's indirect value depends on grain being accessible. The spec requires checking sibling input accessibility.
7. Deterministic tie-breaking: iterate recipes in registry order, prefer lower depth, then stable identity.

## Architecture Check

1. The propagation is a bounded recipe-closure analysis, not a planner search. It uses `recipe_opportunity_depth` as a hard limit, `indirect_value_decay_per_step` as per-step discount, and deterministic iteration order. This keeps it tractable and avoids duplicating planner logic.
2. Multi-input handling prevents a single irrelevant input from inheriting full output value — aligns with Principle 3 (concrete state over abstract scores).
3. No backward-compatibility shims. The stub `indirect_recipe_score: 0` from ticket 003 is replaced with real propagation.

## Verification Layers

1. Commodity gains indirect value when it's an input to a known recipe with valuable output -> focused unit test
2. No indirect value when recipe's workstation is not believed reachable -> focused unit test
3. No indirect value when recipe is not in agent's `KnownRecipes` -> focused unit test
4. Multi-input: input gets value only when sibling inputs are accessible -> focused unit test
5. Multi-step: value propagates through 2+ recipe edges with decay -> focused unit test
6. Depth limit: propagation stops at `recipe_opportunity_depth` -> focused unit test
7. Deterministic: same inputs → same output, registry order → stable tie-breaking -> focused unit test
8. No runaway inflation: best path wins, overlapping paths are not summed -> focused unit test

## What to Change

### 1. Implement indirect recipe value in `commodity_opportunity.rs`

Replace the stub `indirect_recipe_score: 0` with bounded recipe-closure propagation:

```
for each recipe in RecipeRegistry (deterministic order):
    if recipe not in agent's KnownRecipes → skip
    if recipe's workstation not believed reachable within recipe_place_horizon → skip
    if commodity is not an input to this recipe → skip

    compute output_value = max direct value of recipe outputs
        (survival + treatment + enterprise from CommodityOpportunityBreakdown)

    check sibling inputs:
        for each other input in recipe:
            if not held and not accessible via shallower bounded opportunity → skip recipe

    propagated_value = output_value * (1000 - indirect_value_decay_per_step) / 1000

    if depth < recipe_opportunity_depth:
        recurse: check if outputs are themselves recipe inputs (multi-step)
        apply decay at each step

    track best_indirect_value = max across all valid recipes (not sum)
```

### 2. Workstation reachability check

Implement a helper that checks whether any believed workstation with the required tag is reachable within `recipe_place_horizon` hops from the actor's current believed place. Use `GoalBeliefView` place/topology methods.

### 3. Sibling input accessibility check

Implement a helper that checks whether each non-target input of a recipe is either:
- held by the actor (in `holdings` map)
- available locally (in `local_alternatives` map)
- itself satisfiable via a shallower recipe opportunity (recursive, bounded by depth)

### 4. Deterministic tie-breaking

When multiple recipes could provide indirect value for the same commodity:
- iterate in registry order (Vec index)
- prefer lower propagation depth
- take the maximum value, do not sum

## Files to Touch

- `crates/worldwake-sim/src/commodity_opportunity.rs` (modify — replace stub with propagation logic)

## Out of Scope

- Integration with `evaluate_trade_bundle` (ticket 005)
- AI ranking replacement (ticket 006)
- Golden tests (ticket 007)
- Multi-commodity bundle negotiation
- Tool/unique-item requirements for recipes (simplify: ignore `required_tool_kinds` for now, or treat as always-satisfied)

## Acceptance Criteria

### Tests That Must Pass

1. Hungry baker: firewood has `indirect_recipe_score > 0` when baker knows bread recipe, has grain, and mill is reachable
2. Same baker: firewood has `indirect_recipe_score = 0` when no reachable mill is believed
3. Multi-input: firewood has `indirect_recipe_score = 0` when grain is not held and not locally available
4. Multi-step: raw material gains indirect value through 2 recipe steps when within depth limit
5. Depth limit: no indirect value when chain depth exceeds `recipe_opportunity_depth`
6. Decay: indirect value at depth 2 is less than at depth 1 by `indirect_value_decay_per_step`
7. Best-path: when two recipes use the same input, the higher-value path wins (not summed)
8. No-recipes agent: `indirect_recipe_score = 0` when `KnownRecipes` is empty
9. Deterministic: identical inputs produce identical outputs across multiple calls
10. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Indirect value is always traceable to a concrete recipe/output opportunity — no abstract score.
2. Propagation terminates: bounded by `recipe_opportunity_depth` (max 255 via `NonZeroU8`).
3. No floating-point arithmetic — all computation uses `u32`/`Permille`.
4. No stored cache — all values derived at query time.
5. Deterministic iteration: recipes in registry order, `BTreeMap` for holdings/alternatives.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/commodity_opportunity.rs` (extend `#[cfg(test)] mod tests`) — unit tests for indirect recipe propagation, multi-input, multi-step, depth limit, decay, tie-breaking

### Commands

1. `cargo test -p worldwake-sim -- commodity_opportunity` — targeted tests
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite
