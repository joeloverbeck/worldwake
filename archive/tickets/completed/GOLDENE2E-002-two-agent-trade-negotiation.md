# GOLDENE2E-002: Two-Agent Trade Negotiation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Possible
**Deps**: None

## Problem

The Trade domain (`trade_actions`, `trade_valuation`, `MerchandiseProfile`, `DemandMemory`, `TradeDispositionProfile`) has zero golden coverage. This is an entire system crate module (`worldwake-systems/src/trade_actions.rs`) untested at the E2E level. The current architecture models trade as a buyer-driven `AcquireCommodity` plan that resolves into a `trade` action against a seller discovered through `MerchandiseProfile`; it does not currently model two concurrent trade goals.

**Coverage gap filled**:
- GoalKind: `AcquireCommodity { purpose: SelfConsume }` executed via the Trade action domain rather than harvest/loot/owned consumption
- ActionDomain: Trade (fully untested)
- Cross-system chain: Buyer need pressure → seller discovery from `MerchandiseProfile` → trade affordance enumeration → valuation/acceptance → goods exchange → conservation

## Assumption Reassessment (2026-03-12)

1. `MerchandiseProfile` exists in `crates/worldwake-core/src/trade.rs` — defines what an agent sells (confirmed).
2. `TradeDispositionProfile` exists in `crates/worldwake-core/src/trade.rs` — defines trade willingness parameters (confirmed).
3. `DemandMemory` exists in `crates/worldwake-core/src/trade.rs` — records observed demand (confirmed).
4. Trade action handler in `crates/worldwake-systems/src/trade_actions.rs` handles the exchange mechanics (confirmed).
5. `candidate_generation.rs` in `worldwake-ai` emits `RestockCommodity` and `MoveCargo` enterprise goals, but explicitly does not emit `SellCommodity` yet (confirmed by `still_deferred_goal_kinds_are_not_emitted`).
6. `search.rs` and `goal_model.rs` already support buyer-side trade planning: an `AcquireCommodity` goal can compile to a travel-then-trade plan with a concrete trade payload override (confirmed).
7. `trade_actions.rs` enumerates trade payloads from the counterparty seller's `MerchandiseProfile`, the buyer's coin holdings, and mutual valuation acceptance; this means the seller participates through state and affordances, not a separately generated `SellCommodity` goal (confirmed).
8. `DemandMemory` is relevant to enterprise/restock behavior, but not required for the minimal buyer-driven trade path this ticket should cover (confirmed).

## Architecture Check

1. Forcing this ticket to prove a seller-side `SellCommodity` goal would not validate the current architecture; it would require a broader AI/enterprise design change across candidate generation, ranking, interrupt handling, and failure handling.
2. The cleaner and more robust scope is to prove the architecture that exists today: buyer-side trade acquisition through shared state and affordances. That gives real golden coverage of the trade domain without introducing special cases or fake dual-goal choreography.
3. A new test file `golden_trade.rs` is still warranted because Trade has distinct setup and assertions.
4. No backwards-compatibility shims or alias paths are needed.

## What to Change

### 1. Add focused golden trade coverage

Create `crates/worldwake-ai/tests/golden_trade.rs`.

**Test: `golden_buyer_driven_trade_acquisition`**

Setup:
- Seller at Village Square with bread, a `MerchandiseProfile` listing bread for sale, and enough trade configuration for the action semantics involved in the scenario.
- Buyer at Village Square, hungry, with coins, no food, and a `TradeDispositionProfile` so the buyer can execute the trade action.
- No manual action queueing or direct trade invocation.

Expected emergent chain:
1. Buyer generates `AcquireCommodity { commodity: Bread, purpose: SelfConsume }` from hunger.
2. Planner resolves the goal through the Trade domain because a colocated seller advertises bread through `MerchandiseProfile`.
3. Trade executes through the real action pipeline; bread transfers from seller to buyer and coins transfer from buyer to seller.
4. Conservation holds for bread and coins at every observed tick.
5. Buyer hunger decreases after the acquired bread is consumed.

### 2. Keep harness changes minimal

If trade setup becomes repetitive, add a small helper in `golden_harness/mod.rs` for applying `MerchandiseProfile` / `TradeDispositionProfile` to an already-seeded agent. Do not add a merchant-specific seeding abstraction unless it clearly removes duplication across multiple trade tests.

### 3. Engine Changes Made

- Fix `worldwake-ai/src/search.rs` so terminal successors are considered before beam truncation. This ticket exposed a planner-policy bug where a valid local trade barrier could be pruned behind cheaper nonterminal travel branches, making buyer-side trade unreachable under the default planning budget even though candidate generation, affordance construction, and hypothetical transition application all succeeded.
- Add a focused unit regression in `search.rs` covering a local trade barrier surrounded by many cheaper travel branches.

### 4. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P2 (Two-Agent Trade Negotiation) from Part 3 to Part 1.
- Update Part 2: Trade ActionDomain now tested.
- Keep `SellCommodity` marked untested; this ticket does not add seller-side goal generation coverage.

## Files to Touch

- `crates/worldwake-ai/tests/golden_trade.rs` (new — trade domain golden tests)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (optional, minimal helper only if needed)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Multi-hop trade routes (covered by GOLDENE2E-008)
- Merchant restock from production (covered by GOLDENE2E-008)
- Price negotiation complexity beyond basic valuation
- Trade rejection scenarios

## Engine Discovery Protocol

This ticket is a golden e2e test that exercises emergent behavior through the real AI loop.
If implementation reveals that the engine cannot produce the expected emergent behavior,
the following protocol applies:

1. **Diagnose**: Identify the specific engine limitation (missing candidate generation path, planner op gap, action handler deficiency, belief view gap, etc.).
2. **Do not downgrade the test**: The test scenario defines the desired emergent behavior. Do not weaken assertions or remove expected behaviors to work around engine gaps.
3. **Fix forward**: Implement the minimal, architecturally sound engine change that enables the emergent behavior. Document the change in a new "Engine Changes Made" subsection under "What to Change". Each fix must:
   - Follow existing patterns in the affected module
   - Include focused unit tests for the engine change itself
   - Not introduce compatibility shims or special-case logic
4. **Scope guard**: If the required engine change exceeds this ticket's effort rating by more than one level (e.g., a Small ticket needs a Large engine change), stop and apply the 1-3-1 rule: describe the problem, present 3 options, recommend one, and wait for user confirmation before proceeding.
5. **Document**: Record all engine discoveries and fixes in the ticket's Outcome section upon completion, regardless of whether fixes were needed.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_buyer_driven_trade_acquisition` passes under the real AI loop.
2. Seller's bread quantity decreases after trade.
3. Buyer's bread quantity increases after trade.
4. Seller's coin quantity increases after trade.
5. Buyer's coin quantity decreases after trade.
6. Conservation: total bread and coin quantities never increase across the simulation.
7. Buyer's hunger decreases after the trade-acquired bread is consumed.
8. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: Trade ActionDomain marked as tested; `SellCommodity` remains untested.
9. Existing suite: `cargo test -p worldwake-ai --test golden_trade`
10. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation holds for all commodity kinds every tick
3. Determinism: same seed produces same outcome
4. Both agents must be alive throughout the test
5. The test must validate the existing buyer-driven trade architecture, not an invented concurrent seller-goal path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_trade.rs::golden_buyer_driven_trade_acquisition` — proves buyer-driven AI trade acquisition against a seller advertised through `MerchandiseProfile`
2. `crates/worldwake-ai/tests/golden_trade.rs::golden_buyer_driven_trade_acquisition_replays_deterministically` — proves deterministic replay for the trade-domain golden scenario
3. `crates/worldwake-ai/src/search.rs::search_prefers_local_trade_barrier_over_cheaper_nonterminal_travel_options` — prevents the planner from pruning a valid local trade barrier behind cheaper irrelevant branches

### Commands

1. `cargo test -p worldwake-ai --test golden_trade golden_buyer_driven_trade_acquisition`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - Added `crates/worldwake-ai/tests/golden_trade.rs` with a buyer-driven trade golden scenario plus deterministic replay coverage.
  - Updated `crates/worldwake-ai/src/search.rs` so terminal successors are selected before beam truncation, fixing a planner-policy bug that made valid local trade barriers unreachable under the default search budget.
  - Added a focused search regression proving local trade is not pruned behind cheaper nonterminal travel branches.
  - Updated `reports/golden-e2e-coverage-analysis.md` to record Trade coverage and the new golden file/test counts while keeping `SellCommodity` untested.
- Deviations from original plan:
  - The final scenario validates bread trade rather than apples because the implemented buyer-side trade path and existing valuation assumptions are already exercised around bread.
  - The ticket no longer claims seller-side `SellCommodity` coverage; the architecture under test is buyer-driven acquisition against seller affordances.
  - No harness helper or `GENERAL_STORE` constant was needed; the trade setup stayed localized to the new golden test file.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_trade`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
