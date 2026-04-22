# S114PLASTGUA-013: Remote acquisition belief must distinguish displayed sale stock from loose cargo

**Status**: PENDING
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
5. `BelievedEntityState` in [belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs:1554) stores kind, place, inventory, artifact/contention/evidence state, etc., but no custody/container/sale-specific structure for item lots. Remote `entities_at(place)` therefore knows a lot is at a place, but remote `direct_container` / `direct_possessor` cannot distinguish displayed sale stock from loose ground cargo.
6. `local_unpossessed_commodity_evidence()` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs:5136) treats a lot as loose cargo when `direct_container(entity)` and `direct_possessor(entity)` are both `None`. For remote beliefs that omit container/possessor state, displayed stock is misclassified as loose cargo and can route into `MoveCargo`.
7. `listed_sale_lots_at()` / `seller_for_sale_lot()` in [per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs:1676) mix believed entity-place membership with authoritative `SaleListing` / `StockAssignment` / facility-controller reads. This partially surfaces sale stock remotely, but not enough to suppress the competing loose-cargo interpretation.
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

### 1. Make remote belief/state preserve the needed sale-custody distinction

Audit the smallest lawful remote carrier that can distinguish:

- displayed sale stock
- loose ground cargo
- directly possessed cargo

Land that distinction on the belief-visible path without granting forbidden omniscient reads.

### 2. Update acquisition evidence to respect the new distinction

`candidate_generation.rs` acquisition helpers must stop treating remotely believed displayed sale stock as loose cargo. Seller-backed sale evidence should route toward `trade`, while only truly loose cargo should support `MoveCargo`.

### 3. Add focused planner proof before reopening the golden

Add focused candidate-generation / search coverage for the exact reproduced merchant setup so the next golden ticket can depend on a real planner contract instead of ticket prose.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — if the belief carrier needs widening)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — remote economic read semantics)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — acquisition evidence classification)
- `crates/worldwake-ai/src/search/tests.rs` or `crates/worldwake-ai/src/candidate_generation.rs` tests (modify — focused proof)

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

1. `cargo test -p worldwake-ai --lib search::tests::search_returns_travel_then_trade_barrier_for_reachable_seller -- --exact`
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh`
