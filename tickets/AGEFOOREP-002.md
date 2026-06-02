# AGEFOOREP-002: Merchant restocks and restages substitute market food under spoilage

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — `worldwake-ai` enterprise candidate generation/ranking and market stock lifecycle for `RestockCommodity`/`SellCommodity`
**Deps**: Follows from `archive/specs/S178-perishable-food-spoilage.md` and `archive/tickets/AGEFOOREP-001.md`.

## Problem

`scenarios/survival-trade.ron` proves a local substitute-market branch: the buyer starts hungry with coin, the merchant lists Apple rather than Bread, and the buyer acquires Apple by trade before eating. With S178 spoilage enabled, the scenario still exceeds the authored hunger critical-run envelope because the market supply branch is finite unless the merchant lawfully replenishes and restages sale stock from the remote orchard.

The current `commodity_perish_profile: {}` opt-out in `scenarios/survival-trade.ron` is temporary scenario containment, not completion. The founded behavior must make market supply viable through concrete stock, source/sink accounting, listings, beliefs, and action traces.

## Assumption Reassessment (2026-06-02)

1. The motivating golden is `scenarios::survival_trade::survival_trade_proves_substitute_market_branch` in `crates/worldwake-ai/tests/scenarios/survival_trade.rs`.
2. The live scenario has a remote Apple source at `South Orchard Row` and a merchant whose `merchandise_profile.sale_kinds` includes Apple, but the proved buyer branch is local substitute trade rather than direct buyer harvest.
3. The live `GoalKind` surfaces under audit are `RestockCommodity { commodity: Apple }`, `SellCommodity { commodity: Apple }`, and buyer `AcquireCommodity { commodity: Apple, purpose: SelfConsume }`.
4. The shared abstraction boundary is market supply state: `MerchandiseProfile`, `SaleListing`, stock/display containers, `DemandObservation`, and planner-visible beliefs about seller stock/listings.
5. FOUNDATIONS alignment: the fix must preserve FND-3/FND-4 concrete stock and source/sink paths, FND-14B belief-backed planner inputs for remote source/listing state, FND-20 reusable agent reasoning rather than scenario rails, and FND-25A listing lifecycle/actionability.
6. Reassessment exposed this as a separate bug from AGEFOOREP-001: anonymous harvest-source workstation tagging fixes direct harvest execution, while this ticket owns merchant-side restock/restage under spoilage.

## Architecture Check

Merchant replenishment should be an ordinary enterprise behavior: demand or depleted display creates a restock motive, the merchant plans through known harvest/acquisition paths, the resulting goods move through authoritative stock/display state, and listings become actionable only through explicit staging. Do not solve this by increasing starting apples, disabling spoilage permanently, or teaching the buyer to use omniscient remote stock.

## Verification Layers

1. Merchant restock motive appears when listed Apple supply is depleted or spoiled -> decision trace for `RestockCommodity { Apple }` / `ProduceCommodity`.
2. Merchant obtains fresh Apple through lawful source/acquisition path -> action trace and authoritative item/source deltas.
3. Merchant restages saleable Apple -> `stage_stock_for_sale` action trace plus `SaleListing` authoritative state.
4. Buyer purchase remains local and belief-backed -> trade payload trace in `survival_trade`.
5. Survival envelope passes with spoilage enabled -> affected golden and deterministic replay.

## What to Change

### 1. Enterprise Restock Behavior

Audit and update merchant-side candidate generation/ranking/search so depleted or stale sale stock can create a founded restock plan when the merchant has a believed reachable source.

### 2. Market Listing Lifecycle

Ensure restocked Apple can move into display/listed state through existing stock-management actions without preserving stale/actionable listings after their basis ends.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify if ranking suppresses restock)
- `crates/worldwake-ai/src/search/tests.rs` (modify/add focused planner coverage)
- `crates/worldwake-ai/tests/scenarios/survival_trade.rs` (modify if stronger trace assertions are needed)
- `scenarios/survival-trade.ron` (modify to remove temporary opt-out after founded behavior lands)

## Out of Scope

- Theft survival under spoilage; owned by `tickets/AGEFOOREP-003.md`.
- Direct anonymous harvest-source workstation tagging; owned by `archive/tickets/AGEFOOREP-001.md`.

## Acceptance Criteria

### Tests That Must Pass

1. Focused AI/planner coverage proving merchant restock/restage emits and plans from belief-backed Apple supply.
2. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_trade::`
3. Relevant broader AI checks selected during implementation.

### Invariants

1. No planner-visible remote source/listing/state is read outside the actor's belief, local observation, or declared structural substrate.
2. Every new Apple sold after starting stock depletion has an explicit source/sink path and listing lifecycle.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` or `crates/worldwake-ai/src/search/tests.rs` — focused merchant restock/restage proof.
2. `crates/worldwake-ai/tests/scenarios/survival_trade.rs` — existing golden should pass with the opt-out removed once this ticket lands.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_trade::`
3. `cargo test -p worldwake-ai`
