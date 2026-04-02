# S06COMOPPVAL-006: Replace AI bespoke recipe-input ranking with shared layer

**Status**: PENDING
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
7. `RecipeRegistry` is accessible in the AI crate (it's a `worldwake-sim` type, and `worldwake-ai` depends on `worldwake-sim`).

## Architecture Check

1. Removing bespoke recipe-ranking functions and delegating to the shared layer eliminates the architectural split (Design Goal 2). AI and trade now answer "how valuable is this commodity?" through the same code path.
2. Per Principle 28 (no backward compatibility): the old functions are removed, not deprecated or wrapped. No compatibility shim preserving both paths.
3. Candidate generation continues to identify missing recipe inputs directly — only the "how valuable is this input?" question changes to use the shared layer.

## Verification Layers

1. RecipeInput goal ranking uses shared commodity opportunity score -> decision trace
2. ProduceCommodity goal ranking uses shared output opportunity score -> decision trace
3. AI and trade agree on sign of recipe-input value for same belief snapshot -> focused integration test
4. Bespoke functions removed from codebase -> grep verification
5. Existing golden tests pass -> regression (cargo test -p worldwake-ai)

## What to Change

### 1. Replace `recipe_output_priority` calls

Where ranking code calls `recipe_output_priority(recipe_id, context, recipes)` for `RecipeInput` goals, replace with:
- Look up recipe's input commodity
- Call `commodity_opportunity_score` for that commodity
- Use `indirect_recipe_score` (plus direct channels) to derive priority

### 2. Replace `recipe_output_provenance` calls

Where ranking derives provenance for RecipeInput goals, replace with the shared layer's breakdown — `direct_survival_score > 0` means self-consume provenance, `enterprise_score > 0` means enterprise provenance.

### 3. Replace `recipe_output_motive_score` calls

Where ranking scores RecipeInput motive, replace with the sum or max of relevant channels from `CommodityOpportunityBreakdown`.

### 4. Update `ProduceCommodity` ranking

ProduceCommodity goals should also use `commodity_opportunity_score` on the recipe's output commodities to assess how valuable production is.

### 5. Delete bespoke functions

Remove `recipe_output_priority`, `recipe_output_provenance`, `recipe_output_motive_score` from `ranking.rs`. Verify via grep that no other call sites exist.

### 6. Pass `RecipeRegistry` through ranking context

Ensure the ranking context or function parameters include `&RecipeRegistry` so the shared layer can be called. This may require extending a ranking context struct.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — replace bespoke recipe functions with shared layer calls, delete old functions)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — if it references bespoke functions)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — if it references bespoke functions)

## Out of Scope

- Trade valuation integration (ticket 005)
- Golden integration tests (ticket 007)
- Changes to candidate generation logic (candidates still identify missing inputs; only scoring changes)
- Changes to `commodity_opportunity.rs` (tickets 003, 004)

## Acceptance Criteria

### Tests That Must Pass

1. `AcquireCommodity { purpose: RecipeInput(..) }` ranking uses shared commodity opportunity score
2. `ProduceCommodity` ranking uses shared output opportunity score
3. `recipe_output_priority`, `recipe_output_provenance`, `recipe_output_motive_score` no longer exist in the codebase
4. AI ranking and trade valuation agree on the sign of recipe-input value for the same belief snapshot
5. All existing golden tests pass: `cargo test -p worldwake-ai`
6. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. No bespoke recipe-input valuation logic remains in `worldwake-ai` (Principle 28).
2. AI and trade use the same shared `commodity_opportunity_score` for indirect commodity value.
3. Candidate generation still identifies missing recipe inputs — only scoring is unified.
4. All ranking remains deterministic.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (modify existing ranking tests) — update to verify shared-layer integration
2. `crates/worldwake-ai/tests/` (modify golden tests if ranking behavior shifts) — regression verification

### Commands

1. `cargo test -p worldwake-ai -- ranking` — targeted ranking tests
2. `cargo test -p worldwake-ai` — all AI tests including golden (regression)
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite
