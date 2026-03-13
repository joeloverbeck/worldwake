# GOLDE2E-009: AcquireCommodity(RecipeInput) Goal

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `RecipeInput` purpose variant exists but is not currently emitted by candidate generation
**Deps**: None (production and acquire infrastructure exists from E10/E13)

## Problem

`AcquireCommodity { purpose: RecipeInput }` is the only unproven `AcquireCommodity` variant besides `Treatment`. The current golden suite already proves local multi-recipe craft-and-consume when the agent starts with the input (`golden_multi_recipe_craft_path`), but it does not prove the missing-input chain. The real gap is that an agent who needs bread and knows `Bake Bread` cannot yet discover "acquire firewood first" through the AI loop.

## Report Reference

Backlog item **P-NEW-7** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 3).

## Assumption Reassessment (2026-03-13)

1. `CommodityPurpose::RecipeInput(RecipeId)` exists in `worldwake-core/src/goal.rs`.
2. `golden_multi_recipe_craft_path` already covers local `Bake Bread` craft + consume when firewood is pre-seeded; this ticket must not duplicate that scope.
3. `candidate_generation.rs` currently emits `ProduceCommodity` only when the actor already has every required recipe input locally. Missing-input discovery is the actual gap to close.
4. `AcquireCommodity { purpose: RecipeInput(_) }` is currently a modeled goal/satisfaction variant, but no current candidate-generation path emits it.
5. The golden harness already supports custom recipe registries and can stage either a seller-based or world-source-based firewood acquisition path.

## Architecture Check

1. `RecipeInput` acquisition must remain an `AcquireCommodity` path, not a ticket-specific side channel.
2. Missing input goals must be derived from recipe definitions for recipes that actually serve an active higher-level motive (self-consume, restock, treatment), not from hardcoded commodity special cases.
3. `ProduceCommodity` should remain the craft commitment once a satisfiable recipe path exists; missing inputs should be bridged by explicit acquire goals rather than by teaching the planner to "magically" craft through absent inventory.

## Engine-First Mandate

If implementing this e2e suite reveals that `RecipeInput` candidate generation, recipe-input-aware planning, or the acquire→craft→consume chain is incomplete or architecturally unsound — do NOT patch around it. Implement the missing architecture directly and document the actual engine changes in the ticket outcome.

## What to Change

### 1. New golden test in `golden_production.rs`

**Setup**: Agent (Baker) at Village Square, critically hungry. Knows the bake bread recipe (requires firewood → produces bread). Has no firewood. A reachable firewood acquisition path exists. Prefer the narrowest setup that matches the current architecture cleanly:
- seller or local/remote item-lot acquisition is acceptable if it is the cleanest real path today
- a workstation/resource-source path is acceptable only if the current production model already supports it without ticket-specific scaffolding

**Assertions**:
- Agent generates `AcquireCommodity { commodity: Firewood, purpose: RecipeInput }`.
- Agent acquires firewood through the real acquire path.
- Agent then executes the bake bread recipe through the standard production path.
- Agent consumes the crafted bread, reducing hunger.
- Conservation holds throughout (firewood consumed by recipe, bread produced and then consumed).

### 2. Unit coverage in `candidate_generation.rs`

Add focused unit coverage that proves the architectural bridge directly:
- a recipe serving an active higher-level motive emits `AcquireCommodity { purpose: RecipeInput(recipe_id) }` for a missing required input
- `ProduceCommodity` is not emitted until the required inputs are actually satisfiable

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify, if `RecipeInput` generation missing)
- Engine files TBD if planning chain is incomplete

## Out of Scope

- `AcquireCommodity { purpose: Treatment }` (separate gap)
- Multi-input recipes beyond the minimum unit coverage needed to prove aggregation behavior
- Recipe discovery or learning
- Replacing the current production model with a new generic dependency planner

## Acceptance Criteria

### Tests That Must Pass

1. `golden_acquire_commodity_recipe_input` — agent acquires recipe input, crafts, and consumes output
2. Focused unit tests for `RecipeInput` candidate generation and produce-goal suppression until inputs are present
3. Existing suite: `cargo test -p worldwake-ai golden_`
4. Full workspace: `cargo test --workspace`
5. Lint: `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. `RecipeInput` flows through the standard `AcquireCommodity` path
3. `ProduceCommodity` remains the craft goal; missing recipe inputs are bridged by explicit acquire goals
4. Conservation holds: firewood consumed by recipe, bread produced and consumed
5. GoalKind coverage increases: `AcquireCommodity(RecipeInput)` → Yes

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Update GoalKind coverage: `AcquireCommodity (RecipeInput)` → Yes
- Remove P-NEW-7 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_acquire_commodity_recipe_input` — proves RecipeInput acquisition chain
2. `crates/worldwake-ai/src/candidate_generation.rs` unit test(s) — prove missing recipe-input goal emission and produce-goal gating

### Commands

1. `cargo test -p worldwake-ai golden_acquire_commodity_recipe_input`
2. `cargo test -p worldwake-ai candidate_generation`
3. `cargo test -p worldwake-ai golden_`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-13
- Actual changes:
  - candidate generation now emits `AcquireCommodity { purpose: RecipeInput(recipe_id) }` when a relevant craft path is blocked only by missing recipe inputs
  - ranking now scores and prioritizes `RecipeInput` goals from the downstream recipe outputs they unlock, so hunger-driven craft-input acquisition can actually win selection
  - added `golden_acquire_commodity_recipe_input` to prove acquire-input → craft → consume through the real AI loop
  - added focused unit coverage for missing recipe-input goal emission and a search regression proving the trade barrier remains plannable for `RecipeInput`
- Deviations from original plan:
  - the golden scenario uses a local unpossessed firewood lot instead of a seller/travel/source setup
  - this was intentional after reassessment: it proves the missing `RecipeInput` architecture with the narrowest clean real path in the current engine
  - downstream recipe-aware trade valuation was identified as a broader architectural improvement, but it is not required to make `RecipeInput` real for this ticket's scope
- Verification results:
  - `cargo test -p worldwake-ai` passed
  - `cargo test -p worldwake-ai golden_` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
