# S06COMOPPVAL-007: Golden integration tests — baker firewood, seller retention, AI-trade agreement

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S06COMOPPVAL-005, S06COMOPPVAL-006

## Problem

The shared commodity-opportunity layer (tickets 001-006) unifies AI and trade valuation of indirect commodity utility. Without golden integration tests, there is no end-to-end verification that: (a) agents value recipe inputs through the shared layer, (b) sellers retain recipe inputs they need, (c) AI and trade agree on commodity value, and (d) deterministic replay is preserved. The spec lists 9 test scenarios that must be covered.

## Assumption Reassessment (2026-04-02)

1. Golden test files live in `crates/worldwake-ai/tests/`. Relevant existing files: `golden_supply_chain.rs`, `golden_merchant_selling.rs`, `golden_emergent.rs`.
2. Golden tests require `PerceptionProfile` on agents that need to observe (CLAUDE.md).
3. `RecipeRegistry` and `RecipeDefinition` are in `worldwake-sim`. Tests can create registries with test recipes.
4. `CommodityValuationProfile` from ticket 001 must be set on agents that should reason about indirect value.
5. `KnownRecipes` component must include the relevant recipe IDs.
6. Existing golden test infrastructure supports `hash_world`, `hash_event_log`, `verify_authoritative_conservation`, `verify_live_lot_conservation` for determinism and conservation checks.
7. The spec's 9 test scenarios map to these golden tests: baker/firewood (positive + negative), multi-input recipe, multi-step chain, seller retention, demand-vs-recipe, AI-trade agreement, deterministic replay, no-recipes fallback.

## Architecture Check

1. Golden tests follow the established pattern: setup world → run simulation → assert traces and state. No new test infrastructure needed.
2. Replay tests follow the existing pattern: run twice with same seed, compare hashes.
3. No backward-compatibility shims — new tests only.

## Verification Layers

1. Baker values firewood through bread recipe -> decision trace (goal ranking includes RecipeInput)
2. Baker does not value firewood when mill unreachable -> decision trace (no RecipeInput goal or low ranking)
3. Seller retains recipe inputs -> action trace (trade rejected or higher reservation price)
4. AI-trade agreement -> both layers produce same sign of value for same commodity
5. Conservation -> authoritative world state
6. Deterministic replay -> state hash comparison

## What to Change

### 1. Create golden test for baker/firewood valuation

Setup:
- Baker agent at village with hunger, `CommodityValuationProfile`, `KnownRecipes` including bread recipe
- Bread recipe: grain + firewood → bread (requires mill workstation)
- Mill workstation believed reachable
- Baker has grain, no firewood, coins available
- Seller agent with firewood for sale

Assert:
- Baker values firewood (generates `AcquireCommodity { purpose: RecipeInput(bread_recipe) }` or trades for firewood at premium)
- Same setup without reachable mill → baker does NOT value firewood through recipe

### 2. Create golden test for multi-input recipe

Setup:
- Agent knows recipe requiring 2 inputs (A + B → C)
- Agent has input A but not input B, and B is not locally available

Assert:
- Input A does not gain full indirect value from recipe (sibling B unavailable)

### 3. Create golden test for seller retention

Setup:
- Merchant with firewood + grain + reachable mill
- Buyer offers coins for firewood

Assert:
- Merchant refuses or demands higher price for firewood (indirect recipe value raises reservation)

### 4. Create golden test for AI-trade agreement

Setup:
- Same agent, same belief snapshot
- Check commodity_opportunity_score for a recipe input

Assert:
- AI ranking direction matches trade valuation direction for the same commodity

### 5. Create golden test for deterministic replay

- Run baker/firewood scenario twice with same seed
- Compare `hash_world` and `hash_event_log`

### 6. Create test for no-recipes fallback

Setup:
- Agent without `CommodityValuationProfile` or with empty `KnownRecipes`

Assert:
- Valuation uses only direct channels (survival, treatment, enterprise, coin)
- No errors, no indirect recipe value

## Files to Touch

- `crates/worldwake-ai/tests/golden_commodity_opportunity.rs` (new — dedicated golden test file for S06 scenarios)
- `crates/worldwake-ai/tests/golden_harness/` (modify if test helpers need recipe/valuation setup utilities)

## Out of Scope

- Changes to the commodity_opportunity module (tickets 003, 004)
- Changes to trade_valuation or ranking (tickets 005, 006)
- Performance optimization of recipe propagation
- Multi-step chain tests beyond depth 2 (covered by unit tests in ticket 004)

## Acceptance Criteria

### Tests That Must Pass

1. Baker values firewood positively when it closes a believed reachable bread recipe
2. Baker does not value firewood when no reachable mill is believed available
3. Multi-input recipe does not grant full indirect value to one input when sibling is unavailable
4. Seller refuses bundle that would give up last enabling input for a higher-valued recipe opportunity
5. Remembered demand can outweigh recipe retention when enterprise value is concretely higher
6. AI ranking and trade valuation agree on sign of recipe-input value for same belief snapshot
7. Deterministic replay preserves trade acceptance outcomes under indirect recipe valuation
8. No-needs agent without useful recipes evaluates purely from enterprise/coin/wound channels
9. Conservation invariants hold: `verify_authoritative_conservation`, `verify_live_lot_conservation`
10. All existing golden tests pass: `cargo test -p worldwake-ai` — no regressions
11. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Deterministic replay: same seed, same inputs → identical hashes.
2. Conservation: commodities and coins neither created nor destroyed.
3. No regressions in existing golden tests.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_commodity_opportunity.rs` (new) — all 9 spec scenarios as golden E2E tests
2. Replay companion tests for deterministic verification

### Commands

1. `cargo test -p worldwake-ai --test golden_commodity_opportunity` — targeted new tests
2. `cargo test -p worldwake-ai` — all AI golden tests (regression)
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite
