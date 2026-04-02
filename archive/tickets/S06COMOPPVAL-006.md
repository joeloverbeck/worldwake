# S06COMOPPVAL-006: Replace AI bespoke recipe-input ranking with shared layer

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` (ranking.rs — remove bespoke recipe logic, use shared layer)
**Deps**: S06COMOPPVAL-003, S06COMOPPVAL-004

## Problem

`worldwake-ai/src/ranking.rs` has bespoke recipe-input ranking logic: `recipe_output_priority()`, `recipe_output_provenance()`, and `recipe_output_motive_score()` derive RecipeInput goal value by inspecting recipe outputs. This is a separate, shallower valuation path that diverges from trade valuation. With the shared `commodity_opportunity_score` now available, this bespoke logic must be replaced to unify AI and trade valuation (Design Goal 2, Principle 28).

## Assumption Reassessment (2026-04-02)

1. `recipe_output_priority()` at `crates/worldwake-ai/src/ranking.rs` derives priority from recipe outputs by iterating outputs and checking their commodity priority for self-consume.
2. `recipe_output_provenance()` examines recipe outputs and propagates self-consume provenance backward.
3. `recipe_output_motive_score()` uses recipe-output-forward approach to score motive.
4. These functions are called when ranking `GoalKind::AcquireCommodity { purpose: CommodityPurpose::RecipeInput(recipe_id) }` goals.
5. `GoalKind::ProduceCommodity { recipe_id }` ranking also uses recipe output analysis.
6. The shared `commodity_opportunity_score` (tickets 003+004) provides the same information through `indirect_recipe_score` — but from the commodity's perspective rather than the recipe's perspective. The AI needs to map from recipe to its inputs/outputs, then score each via the shared layer.
7. After `S06COMOPPVAL-005`, ranking no longer needs `RecipeRegistry` directly: `commodity_opportunity_score` reads recipes through `GoalBeliefView::{known_recipes, recipe_definition}`. The stale remaining gap is in `worldwake-ai/src/ranking.rs`, which still owns bespoke recipe-output valuation helpers and still threads `RecipeRegistry` unnecessarily.

## Architecture Check

1. Removing bespoke recipe-ranking functions and delegating to the shared layer eliminates the architectural split (Design Goal 2). AI and trade now answer "how valuable is this commodity?" through the same code path.
2. Per Principle 28 (no backward compatibility): the old functions are removed, not deprecated or wrapped. No compatibility shim preserving both paths.
3. Candidate generation continues to identify missing recipe inputs directly — only the "how valuable is this input?" question changes to use the shared layer.

## Verification Layers

1. RecipeInput goal ranking uses shared commodity-opportunity breakdowns on recipe outputs -> focused ranking tests
2. ProduceCommodity ranking uses shared output opportunity score without `RecipeRegistry` plumbing -> focused ranking tests
3. Bespoke functions removed from `ranking.rs` and `rank_candidates` no longer takes `RecipeRegistry` -> compile/test verification
4. Existing AI and workspace suites pass -> regression

## What to Change

### 1. Replace `recipe_output_priority` calls

Where ranking code currently derives RecipeInput priority from bespoke recipe-output helpers, replace it with shared output-commodity opportunity analysis:
- Look up the recipe through `GoalBeliefView::recipe_definition`
- Score each output commodity via `commodity_opportunity_score`
- Derive priority from the strongest shared output breakdown, keeping drive-style promotion only when the shared breakdown says a direct self-care channel is real

### 2. Replace `recipe_output_provenance` calls

Where ranking derives provenance for RecipeInput or ProduceCommodity goals, choose the best output commodity via the shared breakdown. If that winning output has direct survival value, keep the existing drive-oriented provenance on that commodity; otherwise do not preserve an AI-only fallback provenance path.

### 3. Replace `recipe_output_motive_score` calls

Where ranking scores RecipeInput motive, replace the bespoke helper with weighted scoring from the winning shared output breakdown.

### 4. Update `ProduceCommodity` ranking

ProduceCommodity goals should also use `commodity_opportunity_score` on the recipe's output commodities to assess how valuable production is.

### 5. Delete bespoke functions

Remove `recipe_output_priority`, `recipe_output_provenance`, `recipe_output_motive_score` from `ranking.rs`. Verify via grep that no other call sites exist.

### 6. Remove stale `RecipeRegistry` ranking plumbing

Delete the now-stale `RecipeRegistry` parameter from `rank_candidates` and its call sites. Ranking should stay on the `GoalBeliefView` surface that already carries recipe knowledge after `S06COMOPPVAL-005`.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — replace bespoke recipe helpers with shared-layer calls, remove stale `RecipeRegistry` parameter, update focused tests)
- `crates/worldwake-ai/src/goal_explanation.rs` (modify — if needed for the `rank_candidates` signature change)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — if needed for the `rank_candidates` signature change)

## Out of Scope

- Trade valuation integration (ticket 005)
- Golden integration tests (ticket 007)
- Changes to candidate generation logic (candidates still identify missing inputs; only scoring changes)
- Changes to `commodity_opportunity.rs` (tickets 003, 004)

## Acceptance Criteria

### Tests That Must Pass

1. `AcquireCommodity { purpose: RecipeInput(..) }` ranking uses shared commodity-opportunity analysis through recipe outputs
2. `ProduceCommodity` ranking uses the same shared output-opportunity analysis
3. `recipe_output_priority`, `recipe_output_provenance`, and `recipe_output_motive_score` no longer exist in the codebase
4. `rank_candidates` and its live AI call sites no longer take `RecipeRegistry`
5. `cargo test -p worldwake-ai`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` pass

### Invariants

1. No bespoke recipe-input valuation logic remains in `worldwake-ai` (Principle 28).
2. AI and trade use the same shared `commodity_opportunity_score` surface for indirect commodity value.
3. Candidate generation still identifies missing recipe inputs — only scoring is unified.
4. All ranking remains deterministic.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (modify existing ranking tests) — update to verify shared-layer integration and the removed `RecipeRegistry` dependency
2. Existing `worldwake-ai` regression suites — verify no broader AI fallout without inventing new golden coverage here

### Commands

1. `cargo test -p worldwake-ai -- ranking` — targeted ranking tests
2. `cargo test -p worldwake-ai` — all AI tests including golden (regression)
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite

## Outcome

- **Completed**: 2026-04-02
- **What changed**:
  - removed the bespoke `recipe_output_priority`, `recipe_output_provenance`, and `recipe_output_motive_score` helpers from `crates/worldwake-ai/src/ranking.rs`
  - rewired `AcquireCommodity { purpose: RecipeInput(..) }` and `ProduceCommodity` ranking to evaluate recipe outputs through the shared commodity-opportunity analysis already exposed on `GoalBeliefView`
  - preserved drive-style provenance only when the shared breakdown reports a real direct self-care channel, instead of keeping an AI-only fallback recipe path
  - removed the stale `RecipeRegistry` parameter from `rank_candidates` and updated its live AI call sites in `crates/worldwake-ai/src/goal_explanation.rs` and `crates/worldwake-ai/src/agent_tick/observation.rs`
  - updated the focused ranking harness so tests expose recipe knowledge and related belief-surface data through the same runtime-facing contract as production ranking
- **Deviations from original plan**:
  - the originally drafted “AI and trade sign agreement” acceptance claim was broader than this ticket’s actual owned boundary after reassessment; during implementation it was narrowed back to the AI-ranking rewiring this slice really owned, leaving broader end-to-end agreement proof to the remaining S06 golden roadmap
  - a neighboring `RaidTarget` ranking test became stale under the stronger shared valuation path for newly acquired loot, so its lower-layer scoring path was corrected rather than weakening the new recipe-aware ranking behavior
- **Verification results**:
  - `cargo test -p worldwake-ai -- ranking --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
