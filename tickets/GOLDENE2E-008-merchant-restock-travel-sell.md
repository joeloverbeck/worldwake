# GOLDENE2E-008: Merchant Restock-Travel-Sell Loop

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Possible
**Deps**: GOLDENE2E-002 (Two-Agent Trade Negotiation — builds on trade infrastructure)

## Problem

The full merchant enterprise loop (RestockCommodity → travel to source → acquire → MoveCargo back to market → SellCommodity) is the most complex emergent chain possible. It tests the enterprise signals module (`restock_gap`, `deliverable_quantity`), travel for commerce, and the sell action. No golden test exercises the complete merchant lifecycle.

**Coverage gap filled**:
- GoalKind: `RestockCommodity` (completely untested)
- GoalKind: `MoveCargo` (completely untested)
- GoalKind: `AcquireCommodity { purpose: Restock }` (untested purpose variant)
- Cross-system chain: Merchant profile → enterprise signals → restock gap detection → travel to resource → acquire → travel back → sell to buyer

## Assumption Reassessment (2026-03-12)

1. `GoalKind::RestockCommodity { commodity }` exists (confirmed).
2. `GoalKind::MoveCargo { commodity, destination }` exists (confirmed).
3. Enterprise logic in `crates/worldwake-ai/src/enterprise.rs` handles restock gap and opportunity signals (confirmed).
4. `MerchandiseProfile` and `TradeDispositionProfile` in `crates/worldwake-core/src/trade.rs` configure merchant behavior (confirmed).
5. The full loop requires the planner to chain: travel → acquire → travel-back → sell — this is a deep plan (4+ steps), which may test planner depth limits.

## Architecture Check

1. This is the most complex emergent test in the suite — a multi-phase merchant lifecycle. It validates that the enterprise AI module can drive a full commercial loop.
2. Builds on GOLDENE2E-002's trade infrastructure (merchant setup helpers).
3. Goes in `golden_trade.rs` alongside GOLDENE2E-002.

## What to Change

### 1. Write golden test: `golden_merchant_restock_travel_sell`

In `golden_trade.rs`:

Setup:
- Merchant agent at General Store with `MerchandiseProfile` listing apples for sale, but zero apple inventory.
- Buyer agent at General Store, hungry, with coins.
- Orchard Farm has apple resource source (workstation + ResourceSource).
- General Store and Orchard Farm are connected via the prototype topology.

Expected emergent chain:
1. Merchant detects restock gap (merchandise listed but no inventory).
2. `RestockCommodity { commodity: Apple }` goal generated.
3. Merchant travels to Orchard Farm (may require multi-hop).
4. Harvests apples.
5. Travels back to General Store (or sells from current location if buyer follows — but the intent is return-to-market).
6. `SellCommodity` goal activates, trade with buyer occurs.
7. Conservation: total apples + coins maintained.

**Note**: This chain may require a very high tick count (200+) due to travel + harvest + return. The planner depth may also be a limiting factor — Engine Discovery Protocol likely applies.

### 2. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P8 from Part 3 to Part 1.
- Update Part 2: RestockCommodity and MoveCargo GoalKinds tested.

## Files to Touch

- `crates/worldwake-ai/tests/golden_trade.rs` (modify — add test)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — if new helpers needed)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Multiple restock cycles
- Price optimization or dynamic pricing
- Competing merchants
- Merchant route planning across multiple suppliers

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

1. `golden_merchant_restock_travel_sell` — merchant restocks from distant source and sells to buyer
2. Merchant's apple inventory increases (restock occurred) at some point during simulation
3. Trade occurs — buyer receives apples, merchant receives coins
4. Conservation: total commodity quantities never increase
5. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: RestockCommodity and MoveCargo GoalKinds marked as tested
6. Existing suite: `cargo test -p worldwake-ai --test golden_trade`
7. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation holds for all commodity kinds every tick
3. Determinism: same seed produces same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_trade.rs::golden_merchant_restock_travel_sell` — proves full merchant enterprise loop

### Commands

1. `cargo test -p worldwake-ai --test golden_trade golden_merchant_restock_travel_sell`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
