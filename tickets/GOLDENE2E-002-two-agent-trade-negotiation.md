# GOLDENE2E-002: Two-Agent Trade Negotiation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Possible
**Deps**: None

## Problem

The Trade domain (`trade_actions`, `trade_valuation`, `MerchandiseProfile`, `DemandMemory`, `TradeDispositionProfile`) has zero golden coverage. This is an entire system crate module (`worldwake-systems/src/trade_actions.rs`, `worldwake-systems/src/trade.rs`) untested at the E2E level. Two agents with complementary inventory and merchant profiles should negotiate and exchange goods through the real AI loop.

**Coverage gap filled**:
- GoalKind: `SellCommodity` (seller side)
- GoalKind: `AcquireCommodity { purpose: Restock }` or `AcquireCommodity { purpose: SelfConsume }` (buyer side)
- ActionDomain: Trade (fully untested)
- Cross-system chain: Merchant profiles → AI goal generation → trade negotiation → goods exchange → conservation

## Assumption Reassessment (2026-03-12)

1. `MerchandiseProfile` exists in `crates/worldwake-core/src/trade.rs` — defines what an agent sells (confirmed).
2. `TradeDispositionProfile` exists in `crates/worldwake-core/src/trade.rs` — defines trade willingness parameters (confirmed).
3. `DemandMemory` exists in `crates/worldwake-core/src/trade.rs` — records observed demand (confirmed).
4. Trade action handler in `crates/worldwake-systems/src/trade_actions.rs` handles the exchange mechanics (confirmed).
5. `candidate_generation.rs` in worldwake-ai has enterprise/trade-related goal emission — needs verification that sell goals are generated from MerchandiseProfile.
6. `SellCommodity` goal kind exists in `GoalKind` enum (confirmed in `crates/worldwake-core/src/goal.rs`).

## Architecture Check

1. This test requires setting up two agents with complementary merchant profiles — one has surplus apples to sell, the other wants apples. This validates the entire trade pipeline from goal generation through execution.
2. A new test file `golden_trade.rs` is warranted since Trade is a distinct domain with its own setup patterns (merchant profiles, disposition).
3. No backwards-compatibility shims needed.

## What to Change

### 1. Add harness helper: `seed_merchant_agent()`

In `golden_harness/mod.rs`, add a helper that creates an agent with `MerchandiseProfile` and `TradeDispositionProfile` components, extending the existing `seed_agent` pattern.

### 2. Add harness constant: `GENERAL_STORE`

```rust
pub const GENERAL_STORE: EntityId = prototype_place_entity(PrototypePlace::GeneralStore);
```

### 3. Create `golden_trade.rs` test file

New file: `crates/worldwake-ai/tests/golden_trade.rs`

**Test: `golden_two_agent_trade_negotiation`**

Setup:
- Merchant A (seller) at Village Square with `Quantity(5)` apples and a `MerchandiseProfile` listing apples for sale.
- Agent B (buyer) at Village Square, hungry, with coins but no food.
- Both agents have `TradeDispositionProfile` configured for willingness to trade.

Expected emergent chain:
1. Merchant A generates `SellCommodity { commodity: Apple }` goal (from enterprise signals).
2. Agent B generates `AcquireCommodity { commodity: Apple, purpose: SelfConsume }` goal (from hunger).
3. Trade negotiation occurs — apples transfer from A to B, coins transfer from B to A.
4. Conservation: total apple + coin quantities never increase.

### 4. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P2 (Two-Agent Trade Negotiation) from Part 3 to Part 1.
- Update Part 2: Trade ActionDomain now tested, SellCommodity GoalKind now tested.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add `seed_merchant_agent()`, `GENERAL_STORE`)
- `crates/worldwake-ai/tests/golden_trade.rs` (new — trade domain golden tests)
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

1. `golden_two_agent_trade_negotiation` — two agents at same location execute a trade, goods change hands
2. Seller's apple quantity decreases after trade
3. Buyer's apple quantity increases after trade
4. Conservation: total apple and coin quantities never increase across the simulation
5. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: Trade ActionDomain and SellCommodity GoalKind marked as tested
6. Existing suite: `cargo test -p worldwake-ai --test golden_trade`
7. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation holds for all commodity kinds every tick
3. Determinism: same seed produces same outcome
4. Both agents must be alive throughout the test

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_trade.rs::golden_two_agent_trade_negotiation` — proves two-agent trade pipeline

### Commands

1. `cargo test -p worldwake-ai --test golden_trade golden_two_agent_trade_negotiation`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
