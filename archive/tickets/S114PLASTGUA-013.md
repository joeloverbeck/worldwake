# S114PLASTGUA-013: Remote acquisition belief must distinguish displayed sale stock from loose cargo

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — belief/candidate-generation boundary for remote commodity opportunities
**Deps**: `archive/tickets/S114PLASTGUA-006.md`, `archive/tickets/S114PLASTGUA-007.md`, `archive/tickets/S114PLASTGUA-011.md`

## Problem

[`archive/tickets/S114PLASTGUA-010.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/S114PLASTGUA-010.md) attempted to add a golden where a buyer travels toward a merchant, the merchant departs mid-travel, and the buyer's planned `trade` step is invalidated by the trade guard. Live reassessment disproved that golden premise: under the current remote belief surface, displayed sale stock is planner-visible as generic commodity evidence without enough custody/detail to exclude `MoveCargo`, so the autonomous planner does not truthfully hold the authored `Travel -> Trade` branch. In the reproduced setup, the planner selected `Travel -> MoveCargo` when the belief store knew about displayed merchant stock; when the setup removed the cargo-like path, no remote acquisition candidate remained at all.

That means the missing work is not golden-only. The engine still lacks a remote commodity-opportunity belief contract that preserves enough sale-stock structure for `AcquireCommodity(SelfConsume)` to distinguish "merchant-displayed stock that should route through `trade`" from "loose cargo that can route through `MoveCargo`."

## Assumption Reassessment (2026-04-22)

1. Live `trade` registration in [trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs:33) already carries the S114 guard template (`TargetPresent`, `TargetMoved`, `BeliefStatusChange`). The missing behavior is upstream of guard execution: the autonomous planner is not selecting that `trade` step in the remote merchant setup.
2. The attempted golden setup in [`archive/tickets/S114PLASTGUA-010.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/S114PLASTGUA-010.md) used the existing merchant-selling helpers and seeded remote beliefs via `seed_actor_world_beliefs(...)`. With displayed merchant stock, the buyer's live decision trace selected `AcquireCommodity(SelfConsume)` with `selected_plan=Travel->MoveCargo`, not `Travel->Trade`.
3. When the setup was tightened to direct-possession sale stock to eliminate the cargo-like branch, the buyer produced no acquisition candidates at all. That confirmed the planner is depending on remote stock visibility that does not preserve enough sale/custody structure to truthfully route into `trade`.
4. Exact shared abstraction boundary under audit: `BelievedEntityState` / remote `PerAgentBeliefView` economic queries (`entities_at`, `listed_sale_lots_at`, `seller_for_sale_lot`, `direct_container`, `direct_possessor`) as consumed by `candidate_generation.rs` acquisition evidence helpers.
5. Live reassessment showed the minimum sale-stock routing substrate was already present elsewhere: `listed_sale_lots_at()` / `seller_for_sale_lot()` in [per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs:1676) and the mirrored `PlanningState` economic snapshot already preserve seller-backed remote listed lots.
6. The narrower missing detail is on the planner-visible custody lane, not on sale visibility itself. `PerAgentBeliefView` can still discover the remote display container for a known listed lot, but [planning_snapshot.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs:908) intentionally drops `direct_container` / `direct_possessor` for belief-backed remote entities, so search/candidate code cannot rely on those fields to distinguish sale stock from loose cargo.
7. The actual contradiction is in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs:5136): `local_unpossessed_commodity_evidence()` treats any lot with `direct_container(entity) == None` and `direct_possessor(entity) == None` as loose cargo, so planner-visible listed sale stock is double-classified as both seller-backed sale evidence and loose-cargo evidence.
8. Planner contract impact: this is a planner-visible belief-carriage issue, not a golden-only issue. The owned proof must include candidate generation and selected-plan routing before any new golden can honestly claim a remote `trade` barrier.
9. Golden ownership note: [`archive/tickets/S114PLASTGUA-010.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/S114PLASTGUA-010.md) is the rejection record for the disproven remote-trade golden contract; this follow-up owns the substrate gap that must land before any truthful replacement golden exists.

## Architecture Check

1. The clean fix is to make remote commodity-opportunity beliefs preserve the minimum truthful custody/sale structure needed for planner routing, rather than weakening the golden to accept `MoveCargo` as a stand-in for remote trade or adding test-only planner exceptions.
2. This stays aligned with FOUNDATIONS locality and belief-only planning: the planner should act on believed sale/custody facts, not on omniscient remote world queries or on ambiguous lots treated as both sale stock and loose cargo.

## Verification Layers

1. Remote displayed sale stock remains classified as sale stock rather than loose cargo -> focused `PerAgentBeliefView` / belief-carrier test
2. `AcquireCommodity(SelfConsume)` emits a remote seller-backed opportunity that can route through `trade` -> focused candidate-generation test
3. Planner search selects `Travel -> Trade` instead of `Travel -> MoveCargo` for the reproduced merchant setup -> focused search/decision-trace test
4. Existing local merchant-selling and cargo behavior stay lawful -> same-domain regression tests in `golden_merchant_selling` / focused cargo tests
5. Once the planner-visible substrate is live, a new bounded golden ticket can own the end-to-end merchant-departure guard-breach scenario without reopening the archived rejection record.

## What to Change

### 1. Update acquisition evidence to respect the existing sale-stock distinction

`candidate_generation.rs` acquisition helpers must stop treating remotely believed displayed sale stock as loose cargo. Seller-backed sale evidence should route toward `trade`, while only truly loose cargo should support `MoveCargo`.

### 2. Add focused planner proof before reopening the golden

Add focused candidate-generation / search coverage for the exact reproduced merchant setup so the next golden ticket can depend on a real planner contract instead of ticket prose.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — focused proof of the existing remote sale-stock read boundary)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — acquisition evidence classification)
- `crates/worldwake-ai/src/search/tests.rs` and/or `crates/worldwake-ai/src/candidate_generation.rs` (modify — focused proof)

## Out of Scope

- Golden E2E authoring for the merchant-departure guard-breach scenario
- New guard kinds or changes to the `trade` guard template itself
- Broad merchant-enterprise behavior unrelated to remote acquisition belief semantics

## Acceptance Criteria

### Tests That Must Pass

1. Focused belief/economic-view proof that remote displayed sale stock is not misclassified as loose cargo
2. Focused candidate-generation proof that the remote merchant setup emits the seller-backed acquisition opportunity
3. Focused search proof that the setup selects `Travel -> Trade`
4. Existing same-domain merchant/cargo regressions stay green

### Invariants

1. Remote displayed sale stock and loose cargo are no longer planner-equivalent when that distinction changes the lawful operator family
2. The fix preserves belief-only planning; it does not reintroduce omniscient remote entity reads

## Test Plan

### New/Modified Tests

1. Focused belief/economic-view test — proves the remote custody/sale distinction
2. Focused candidate-generation/search test — proves `Travel -> Trade` on the reproduced merchant setup

### Commands

1. `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::remote_listed_sale_lot_stays_sale_visible_through_display_container -- --exact`
2. `cargo test -p worldwake-ai --lib candidate_generation::tests::remote_listed_sale_lot_does_not_emit_loose_lot_acquire_evidence -- --exact`
3. `cargo test -p worldwake-ai --lib search::tests::search_returns_travel_then_trade_barrier_for_remote_listed_sale_lot_without_custody_detail -- --exact`
4. `cargo test -p worldwake-ai --lib search::tests::search_returns_travel_then_trade_barrier_for_reachable_seller -- --exact`
5. `cargo test -p worldwake-ai --lib candidate_generation::tests::cargo_candidate_emitted_from_local_stock_and_demand -- --exact`
6. `cargo test -p worldwake-ai --test golden_merchant_selling buyer_trades_against_listed_lot -- --exact`
7. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-04-22.

- Kept the implementation on the truthful live seam: the existing remote sale-stock lane (`listed_sale_lots_at` / `seller_for_sale_lot`) already carried enough information for trade routing, while `local_unpossessed_commodity_evidence()` was still reclassifying those same listed lots as loose cargo whenever planner-visible custody was absent.
- Updated `crates/worldwake-ai/src/candidate_generation.rs` so seller-backed listed lots are excluded from loose-cargo evidence. This keeps remote displayed sale stock anchored to `trade` instead of letting it also feed `MoveCargo`.
- Added a focused `worldwake-sim` proof that a remote known listed lot remains sale-visible through the display-container lane, a focused candidate-generation proof that the reproduced remote acquire opportunity keeps seller-only evidence, and a focused search proof that the same seller-backed remote setup now selects `Travel -> Trade` without requiring remote custody detail in the search harness.

## Deviations

- Reassessment narrowed the ticket from “widen the belief carrier if needed” to “respect the already-landed seller/listing carrier and stop double-classifying it as loose cargo.” No `BelievedEntityState` schema change was needed.
- The strongest honest belief-carrier proof stayed in `per_agent_belief_view.rs`, but the decisive planner proof required explicitly seeding the remote seller and lot into the search harness’ known-entity beliefs so `PlanningSnapshot` could lawfully admit the remote listed lot. That matches the live planner admission contract instead of assuming remote entities are globally visible.

## Verification Result

- Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::remote_listed_sale_lot_stays_sale_visible_through_display_container -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::remote_listed_sale_lot_does_not_emit_loose_lot_acquire_evidence -- --exact`
- Passed `cargo test -p worldwake-ai --lib search::tests::search_returns_travel_then_trade_barrier_for_remote_listed_sale_lot_without_custody_detail -- --exact`
- Passed `cargo test -p worldwake-ai --lib search::tests::search_returns_travel_then_trade_barrier_for_reachable_seller -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::cargo_candidate_emitted_from_local_stock_and_demand -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling buyer_trades_against_listed_lot -- --exact`
- Passed `cargo test -p worldwake-ai`
