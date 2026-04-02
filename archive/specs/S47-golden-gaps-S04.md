**Status**: ✅ COMPLETED

# S47: Golden Gap — Hungry Merchant Eats Own Listed Sale Stock

## Summary

Post-implementation golden gap analysis for S04 (Merchant Selling Market Presence). One cross-system emergent scenario identified: a hungry merchant autonomously consumes their own listed bread, demonstrating that survival pressure naturally overrides enterprise goals without any special merchant-vs-needs logic.

## Scenario: Hungry Merchant Eats Own Listed Sale Stock

A merchant at `home_market` has listed bread for sale via `staff_market`. Critical hunger drives the merchant to eat their own sale stock, removing the `SaleListing` as a side effect of consumption.

### Description

1. Merchant is at home_market with a bread lot that has `SaleListing`
2. Merchant has critical hunger (`pm(950)+`)
3. AI generates both `ConsumeOwnedCommodity { Bread }` (survival) and `SellCommodity { Bread }` (enterprise)
4. Survival-class priority wins ranking — merchant eats bread
5. After consumption: bread lot quantity decreases or lot is archived, `SaleListing` is removed

### GoalKinds Exercised

- `ConsumeOwnedCommodity { commodity: Bread }` — survival goal targeting the sale lot
- `SellCommodity { commodity: Bread }` — enterprise goal competing for the same lot

### ActionDomains Exercised

- `Needs` — `eat` action consuming the listed lot
- `Trade` — `staff_market` action (if it starts before hunger becomes critical) and `SaleListing` lifecycle

### Systems Exercised

- **Needs (E09)**: Generates hunger-driven consumption goal
- **Trade (S04)**: `SaleListing` component, `SellCommodity` candidate generation, listing cleanup
- **AI Ranking (E13)**: Survival vs enterprise priority ordering via `GoalPriorityClass`

### Setup Requirements

- One merchant agent at `VILLAGE_SQUARE` (home_market)
- `MerchandiseProfile` with `sale_kinds: [Bread]`, `home_market: VILLAGE_SQUARE`
- One bread lot with `SaleListing { listed_at: Tick(0) }` — directly possessed by merchant
- Critical hunger: `HomeostaticNeeds { hunger: pm(950), ... }`
- `TradeDispositionProfile` with `market_presence_ticks`
- `PerceptionProfile` for observation
- `DemandMemory` for nonzero enterprise motive
- Demand memory and enterprise_weight should be set so `SellCommodity` would be selected if hunger were not critical

### What Emergence It Demonstrates

The merchant's autonomous needs-driven consumption of their own trade stock demonstrates Principle 1 (Maximal Emergence): the survival-over-enterprise priority ordering is a consequence of general-purpose goal ranking rules, not a special "merchant hunger override" system. The physical coupling — where the survival action directly consumes the trade-listed lot — is what distinguishes this from Scenario 44 (pain vs enterprise on different entities).

### Foundation Principle Alignment

- **Principle 1** (Maximal Emergence): Merchant eating sale stock emerges from general needs + ranking rules
- **Principle 3** (Concrete State Over Abstract Scores): Hunger is a concrete need value, the lot is a concrete entity with both `ItemLot.commodity` and `SaleListing`
- **Principle 20** (Agent Diversity): Different `UtilityProfile` weights would produce different survival-vs-enterprise tradeoff outcomes

### Why It Is Not a Duplicate

- **Scenario 44** (wounded politician): Tests pain vs enterprise priority, but the survival action (`TreatWounds`) and the enterprise action (`ClaimOffice`) target entirely different entities. In this scenario, the survival action directly consumes the enterprise asset — the lot itself.
- **Existing `golden_merchant_selling.rs` tests**: Cover listing lifecycle, buyer trade, dampening, ranking — but never needs-driven consumption of a listed lot.
- **Not in rejected scenarios**: The old rejection (#4 in coverage dashboard) was about missing emission logic, not about needs-vs-trade interaction.

## Ticket Breakdown

### S47GOLGAP-001: Hungry merchant eats own listed sale stock golden test

- Add `hungry_merchant_eats_listed_stock` test to `golden_merchant_selling.rs`
- Add `hungry_merchant_eats_listed_stock_replays_deterministically` replay companion
- Setup: single merchant, critical hunger, listed bread, demand memory
- Assert: (a) merchant eats bread (action trace), (b) SaleListing removed after consumption (world state), (c) hunger decreases (world state)
- Proof surfaces: action trace (eat committed), authoritative world state (SaleListing gone, need decreased)

**Files**: `crates/worldwake-ai/tests/golden_merchant_selling.rs`
**Effort**: Small

## Tests

- [ ] hungry merchant eats own listed stock — survival overrides enterprise, SaleListing removed
- [ ] deterministic replay companion

## Acceptance Criteria

1. Merchant with critical hunger and listed bread eats the bread
2. `SaleListing` is removed after the lot is consumed (archived or quantity reduced to 0)
3. Conservation invariant holds: `verify_live_lot_conservation` passes
4. Deterministic replay produces identical world and event log hashes
5. Existing `golden_merchant_selling.rs` tests continue to pass

## Outcome

- **Completion date**: 2026-04-02
- **What changed**: Golden test `hungry_merchant_eats_listed_stock` and replay companion implemented in `golden_merchant_selling.rs`. Merchant with critical hunger consumes own listed bread, SaleListing removed as side effect.
- **Deviations from original plan**: None known.
- **Verification**: All golden tests pass, conservation invariants hold, deterministic replay verified.
