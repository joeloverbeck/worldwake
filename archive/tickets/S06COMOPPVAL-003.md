# S06COMOPPVAL-003: Shared commodity_opportunity module — direct value channels

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` (new module with shared valuation logic)
**Deps**: S06COMOPPVAL-001, S06COMOPPVAL-002

## Problem

Trade valuation (`evaluate_trade_bundle`) and AI ranking (`ranking.rs`) currently compute commodity value through separate, incompatible code paths. The direct value channels (survival, treatment, enterprise, coin) are embedded inside `worldwake-sim/src/trade_valuation.rs`'s `snapshot()` function and not reusable by AI. This ticket creates the shared `commodity_opportunity` module with the direct channels factored out, establishing the foundation that ticket 004 extends with indirect recipe value.

## Assumption Reassessment (2026-04-02)

1. `trade_valuation.rs` at `crates/worldwake-sim/src/trade_valuation.rs` computes four channels in `snapshot()`: survival (via `CommodityConsumableProfile`), wound/treatment (via `wound_score()`), demand (via `demand_score()`), and coin. These are currently private to `trade_valuation.rs`.
2. `build_current_holdings()` at `crates/worldwake-sim/src/trade_valuation.rs` returns `BTreeMap<CommodityKind, u32>` — the exact pattern proposed for `commodity_opportunity_score`'s `holdings` parameter.
3. `aggregate_local_alternatives()` at `crates/worldwake-sim/src/trade_valuation.rs` returns `BTreeMap<CommodityKind, u32>` — same pattern for `local_alternatives`.
4. `CommodityKind::spec()` returns `CommodityKindSpec` with `consumable_profile: Option<CommodityConsumableProfile>` and `treatment_profile: Option<CommodityTreatmentProfile>`.
5. `GoalBeliefView` now has `commodity_valuation_profile` (ticket 002), `homeostatic_needs`, `demand_memory`, and `wounds` methods.
6. `RecipeRegistry` at `crates/worldwake-sim/src/recipe_registry.rs` is available. This ticket passes it through but does not use it yet (indirect value = 0).
7. No file `commodity_opportunity.rs` exists in `worldwake-sim/src/`.

## Architecture Check

1. Factoring direct channels into a shared module establishes the shared AI-trade valuation boundary (Design Goal 2) without prematurely widening this ticket into runtime integration. Ticket 005 will switch `trade_valuation.rs` to delegate to the new helper.
2. The module lives in `worldwake-sim` (not `worldwake-systems` or `worldwake-ai`), keeping it accessible to both consumers without circular dependencies.
3. No backward-compatibility shims. This ticket lands the reusable shared helper first; ticket 005 removes the remaining inline trade-only valuation path.

## Verification Layers

1. Direct survival score matches commodity's consumable profile × need pressure -> focused unit test
2. Treatment score matches wound severity × accessible medicine -> focused unit test
3. Enterprise score matches remembered demand -> focused unit test
4. `indirect_recipe_score` returns 0 (placeholder for ticket 004) -> focused unit test
5. All scores are deterministic given identical inputs -> focused unit test
6. Single-layer ticket (new module and exports only; `trade_valuation.rs` delegation remains ticket 005).

## What to Change

### 1. Create `crates/worldwake-sim/src/commodity_opportunity.rs`

Define the public types:

```rust
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct CommodityOpportunityBreakdown {
    pub direct_survival_score: u32,
    pub treatment_score: u32,
    pub enterprise_score: u32,
    pub indirect_recipe_score: u32,
}
```

Implement the scoring function:

```rust
pub fn commodity_opportunity_score(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn GoalBeliefView,
    recipes: &RecipeRegistry,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
) -> CommodityOpportunityBreakdown
```

Direct channel logic (extracted from `trade_valuation.rs` patterns):
- **Survival**: `commodity.spec().consumable_profile` → map relief fields against `belief.homeostatic_needs(actor)` pressure
- **Treatment**: `commodity.spec().treatment_profile` → score against `belief.wounds(actor)` severity
- **Enterprise**: count `belief.demand_memory(actor)` observations for this commodity at this place
- **Indirect recipe**: return 0 (stub — ticket 004 implements this)

All arithmetic uses `u32` — no floats.

### 2. Register module in `worldwake-sim/src/lib.rs`

Add `pub mod commodity_opportunity;` and export `CommodityOpportunityBreakdown` and `commodity_opportunity_score`.

### 3. Unit tests

Comprehensive tests for each direct channel in isolation and combined.

## Files to Touch

- `crates/worldwake-sim/src/commodity_opportunity.rs` (new — shared valuation module)
- `crates/worldwake-sim/src/lib.rs` (modify — register and export module)

## Out of Scope

- Indirect recipe value propagation (ticket 004)
- Integration with `evaluate_trade_bundle` (ticket 005)
- AI ranking replacement (ticket 006)
- Modifying `trade_valuation.rs` to delegate to this module (ticket 005)

## Acceptance Criteria

### Tests That Must Pass

1. Survival score > 0 for consumable commodity when agent has matching need pressure
2. Survival score = 0 for non-consumable commodity
3. Treatment score > 0 for Medicine when agent has wounds
4. Treatment score = 0 when agent has no wounds
5. Enterprise score > 0 when demand memory contains observations for the commodity
6. Enterprise score = 0 when no demand observations
7. `indirect_recipe_score` = 0 always (stub)
8. Scores are deterministic: identical inputs → identical outputs
9. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. All scoring is belief-facing — only `GoalBeliefView` is accessed, never `WorldTxn`.
2. No stored commodity-value cache — all values are derived at query time.
3. No floating-point arithmetic.
4. Deterministic iteration — `BTreeMap` keys are `CommodityKind` (enum with derived `Ord`).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/commodity_opportunity.rs` (new `#[cfg(test)] mod tests`) — unit tests for each direct channel and combined scoring

### Commands

1. `cargo test -p worldwake-sim -- commodity_opportunity` — targeted tests
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite

## Outcome

- Completed: 2026-04-02
- What changed:
  - Added the shared direct-channel valuation module in [commodity_opportunity.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/commodity_opportunity.rs).
  - Added `CommodityOpportunityBreakdown` and `commodity_opportunity_score` with belief-facing direct survival, treatment, and enterprise channels.
  - Kept `indirect_recipe_score` as an explicit `0` stub for [S06COMOPPVAL-004](/home/joeloverbeck/projects/worldwake/tickets/S06COMOPPVAL-004.md).
  - Registered and exported the new shared surface from [lib.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/lib.rs).
  - Added focused deterministic unit coverage for consumable need relief, medicine/wound value, demand-memory value, the indirect stub, and repeatability.
- Deviations from original plan:
  - The ticket originally described the source valuation helper more generically; before implementation it was corrected to the live ownership boundary in `crates/worldwake-sim/src/trade_valuation.rs`.
  - This ticket intentionally landed the shared helper without rewiring `trade_valuation.rs`; that caller migration remains explicitly owned by [S06COMOPPVAL-005](/home/joeloverbeck/projects/worldwake/tickets/S06COMOPPVAL-005.md).
- Verification results:
  - `cargo test -p worldwake-sim commodity_opportunity -- --nocapture`
  - `cargo test -p worldwake-sim belief_view -- --nocapture`
  - `cargo test -p worldwake-sim`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
