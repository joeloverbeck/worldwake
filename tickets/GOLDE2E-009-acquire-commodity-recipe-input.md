# GOLDE2E-009: AcquireCommodity(RecipeInput) Goal

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Possible — `RecipeInput` purpose variant in candidate generation may be incomplete
**Deps**: None (production and acquire infrastructure exists from E10/E13)

## Problem

`AcquireCommodity { purpose: RecipeInput }` is the only untested `AcquireCommodity` variant besides `Treatment`. An agent who wants to bake bread but lacks firewood must first acquire firewood (recipe input) before crafting. This multi-step plan — travel → acquire input → craft → consume — exercises the `RecipeInput` purpose through the real AI loop.

## Report Reference

Backlog item **P-NEW-7** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 3).

## Assumption Reassessment (2026-03-13)

1. `CommodityPurpose::RecipeInput` exists in `worldwake-core/src/goal.rs` — verify.
2. Candidate generation should emit `AcquireCommodity { purpose: RecipeInput }` when an agent knows a recipe but lacks an input — verify in `candidate_generation.rs`.
3. The planner must be able to chain: acquire input → craft → consume.
4. The golden harness can configure recipes with specific input requirements.

## Architecture Check

1. `RecipeInput` acquisition should use the same `AcquireCommodity` planner path as `SelfConsume` — only the purpose tag differs.
2. The planner should discover the input need from recipe definitions, not from hardcoded knowledge.

## Engine-First Mandate

If implementing this e2e suite reveals that `RecipeInput` candidate generation, recipe-input-aware planning, or the multi-step acquire→craft→consume chain is incomplete or architecturally unsound — do NOT patch around it. Instead, design and implement a comprehensive architectural solution. Document any engine changes in the ticket outcome.

## What to Change

### 1. New golden test in `golden_production.rs`

**Setup**: Agent (Baker) at Village Square, critically hungry. Knows the bake bread recipe (requires firewood → produces bread). Has no firewood. A source of firewood exists at a reachable location (e.g., a ChoppingBlock workstation with wood resource).

**Assertions**:
- Agent generates `AcquireCommodity { commodity: Firewood, purpose: RecipeInput }`.
- Agent travels to the firewood source, acquires firewood.
- Agent returns (or crafts locally) and executes the bake bread recipe.
- Agent consumes the crafted bread, reducing hunger.
- Conservation holds throughout (firewood consumed by recipe, bread produced and then consumed).

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify, if `RecipeInput` generation missing)
- Engine files TBD if planning chain is incomplete

## Out of Scope

- `AcquireCommodity { purpose: Treatment }` (separate gap)
- Multi-input recipes (one input is sufficient for this test)
- Recipe discovery or learning

## Acceptance Criteria

### Tests That Must Pass

1. `golden_acquire_commodity_recipe_input` — agent acquires recipe input, crafts, and consumes output
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. `RecipeInput` flows through the standard `AcquireCommodity` path
3. Conservation holds: firewood consumed by recipe, bread produced and consumed
4. GoalKind coverage increases: `AcquireCommodity(RecipeInput)` → Yes

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Update GoalKind coverage: `AcquireCommodity (RecipeInput)` → Yes
- Remove P-NEW-7 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_acquire_commodity_recipe_input` — proves RecipeInput acquisition chain

### Commands

1. `cargo test -p worldwake-ai golden_acquire_commodity_recipe`
2. `cargo test --workspace && cargo clippy --workspace`
