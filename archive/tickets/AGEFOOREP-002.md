# AGEFOOREP-002: Merchant restocks and restages substitute market food under spoilage

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — `worldwake-ai` enterprise candidate generation/ranking and market stock lifecycle for `RestockCommodity`/`SellCommodity`
**Deps**: Follows from `archive/specs/S178-perishable-food-spoilage.md` and `archive/tickets/AGEFOOREP-001.md`.

## Problem

`scenarios/survival-trade.ron` proves a local substitute-market branch: the buyer starts hungry with coin, the merchant lists Apple rather than Bread, and the buyer acquires Apple by trade before eating. With S178 spoilage enabled, the scenario still exceeds the authored hunger critical-run envelope because the market supply branch is finite unless the merchant lawfully replenishes and restages sale stock from the remote orchard.

Before this ticket, the `commodity_perish_profile: {}` opt-out in `scenarios/survival-trade.ron` was temporary scenario containment, not completion. The founded behavior needed to make market supply viable through concrete stock, source/sink accounting, listings, beliefs, and action traces.

## Assumption Reassessment (2026-06-02)

1. The motivating golden is `scenarios::survival_trade::survival_trade_proves_substitute_market_branch` in `crates/worldwake-ai/tests/scenarios/survival_trade.rs`.
2. The live scenario has a remote Apple source at `South Orchard Row` and a merchant whose `merchandise_profile.sale_kinds` includes Apple, but the proved buyer branch is local substitute trade rather than direct buyer harvest.
3. The live `GoalKind` surfaces under audit are `RestockCommodity { commodity: Apple }`, `SellCommodity { commodity: Apple }`, and buyer `AcquireCommodity { commodity: Apple, purpose: SelfConsume }`.
4. The shared abstraction boundary is market supply state: `MerchandiseProfile`, `SaleListing`, stock/display containers, `DemandObservation`, and planner-visible beliefs about seller stock/listings.
5. FOUNDATIONS alignment: the fix must preserve FND-3/FND-4 concrete stock and source/sink paths, FND-14B belief-backed planner inputs for remote source/listing state, FND-20 reusable agent reasoning rather than scenario rails, and FND-25A listing lifecycle/actionability.
6. Reassessment exposed this as a separate bug from AGEFOOREP-001: anonymous harvest-source workstation tagging fixes direct harvest execution, while this ticket owns merchant-side restock/restage under spoilage.

## Architecture Check

Merchant replenishment should be an ordinary enterprise behavior: demand or depleted display creates a restock motive, the merchant plans through known harvest/acquisition paths, the resulting goods move through authoritative stock/display state, and listings become actionable only through explicit staging. Do not solve this by increasing starting apples, disabling spoilage permanently, or teaching the buyer to use omniscient remote stock.

## FOUNDATIONS Reassessment Decision (2026-06-02)

The selected implementation boundary is the founded engine path: preserve spoilage, remove the scenario opt-out, and make merchant restock/restage plus buyer local trade remain lawful through concrete stock, saleable listings, belief-backed planner inputs, and physical economic dampeners. The rejected stale premise is that the golden can be completed by scenario-only containment such as increasing buyer/merchant starting resources, disabling Apple spoilage for this fixture, or adding scenario-specific purchase rails. Verification must therefore prove the authored survival-trade branch with the normal perishable Apple profile enabled.

## Outcome

Completed on 2026-06-02.

The landed fix keeps the survival-trade scenario on the founded engine path. Apple spoilage remains enabled in `scenarios/survival-trade.ron`, and the merchant branch now uses concrete saleable stock, explicit stage/listing lifecycle checks, belief-backed planner snapshots, and dampened trade pricing rather than scenario-only containment.

## Landed Changes

### 1. Enterprise Restock And Sale Stock Behavior

Merchant restock gap calculation now counts saleable lot-level market stock, ignores stale/spoiled display stock as available sale supply, and can replace non-saleable listed stock through the normal restock motive. Planner candidate/search coverage also keeps `SellCommodity { Apple }`, `RestockCommodity { Apple }`, and buyer `AcquireCommodity { Apple, SelfConsume }` tied to concrete local sale lots and saleable freshness state.

### 2. Market Listing And Trade Lifecycle

Trade validation, listing pruning, stock staging, and hypothetical planning now reject or ignore stale/spoiled sale lots as actionable market stock. Buyer trade payload synthesis scales requested quantity from concrete sale-lot evidence and affordable units, while seller/buyer pricing uses explicit physical dampeners: current stock, actual unmet demand, local alternatives, listed counterparty stock, and bounded negotiation patience.

### 3. Planner Snapshot And Consumption Accounting

Planner snapshots now carry lot freshness, lot condition, and commodity perish profiles so perishable stock can be reasoned about without authoritative omniscience. Hypothetical self-consumption also applies lot condition to the simulated hunger relief, preserving the same concrete-state semantics used by authoritative consumption.

## Landed Files

- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/effect_sink_hypothetical.rs`
- `crates/worldwake-ai/src/enterprise.rs`
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/goal_schema.rs`
- `crates/worldwake-ai/src/planning_snapshot.rs`
- `crates/worldwake-ai/src/planning_state.rs`
- `crates/worldwake-ai/src/ranking.rs`
- `crates/worldwake-ai/src/search/candidates.rs`
- `crates/worldwake-ai/src/search/mod.rs`
- `crates/worldwake-ai/src/search/tests.rs`
- `crates/worldwake-ai/src/search/transition.rs`
- `crates/worldwake-ai/tests/scenarios/survival_trade.rs`
- `crates/worldwake-systems/src/stock_actions.rs`
- `crates/worldwake-systems/src/trade.rs`
- `crates/worldwake-systems/src/trade_actions.rs`
- `scenarios/survival-trade.ron`

## Out of Scope

- Theft survival under spoilage; owned by `tickets/AGEFOOREP-003.md`.
- Direct anonymous harvest-source workstation tagging; owned by `archive/tickets/AGEFOOREP-001.md`.

## Acceptance Result

The focused AI/planner coverage landed across enterprise restock-gap tests, candidate generation tests, search tests, goal payload tests, and planning-state/hypothetical-state tests. The survival-trade golden suite passed with the perish-profile opt-out removed.

## Invariants Result

1. Planner-visible remote source/listing/state remains belief-backed through the planning snapshot and candidate evidence path; no scenario rail or omniscient buyer remote-stock path was added.
2. Apples sold after starting-stock depletion travel through explicit stock, staging, listing, trade, and consumption state; stale/spoiled lots are not actionable sale stock.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib search::tests::sell_search_does_not_restage_non_saleable_unlisted_stock -- --exact`
- Passed `cargo test -p worldwake-ai --lib search::tests::sell_search_for_stored_home_stock_requires_stage_before_goal_satisfaction -- --exact`
- Passed `cargo test -p worldwake-ai --lib search::tests::search_returns_travel_then_trade_barrier_for_remote_listed_sale_lot_without_custody_detail -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::candidate_gen_caps_perishable_self_consume_acquisition_to_fresh_horizon -- --exact`
- Passed `cargo test -p worldwake-ai --lib goal_model::tests::acquire_goal_trade_payload_scales_to_desired_target -- --exact`
- Passed `cargo test -p worldwake-ai --lib goal_model::tests::acquire_goal_trade_payload_resynthesizes_one_unit_affordance_for_desired_target -- --exact`
- Passed `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_trade::survival_trade_proves_substitute_market_branch -- --ignored --exact`
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_trade::`
- Passed `cargo test -p worldwake-systems`
