# FND02-002: Preserve `SellCommodity` Deferral Until S04

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Tests only — candidate generation regression coverage
**Deps**: Phase 2 complete; S04 remains future work

## Problem

This ticket was drafted under a stale assumption: that `GoalKind::SellCommodity` should already be emitted from `crates/worldwake-ai/src/candidate_generation.rs`.

Current code and spec state do not support that change cleanly:

1. `crates/worldwake-ai/src/search.rs` explicitly treats `GoalKind::SellCommodity` as unsupported.
2. `crates/worldwake-ai/src/candidate_generation.rs` already contains a regression test asserting deferred goal kinds like `SellCommodity` are not emitted.
3. `specs/S04-merchant-selling-market-presence.md` defines the architecture that makes `SellCommodity` real: concrete `SaleListing` state, `staff_market`, listed-lot discovery, and replacement of profile-only seller visibility.

Emitting `SellCommodity` now would create a ranked but unsatisfiable goal on top of the very abstract seller model that S04 is intended to replace.

## Assumption Reassessment (2026-03-13)

1. `GoalKind::SellCommodity { commodity: CommodityKind }` exists in `crates/worldwake-core/src/goal.rs` — confirmed.
2. `crates/worldwake-ai/src/ranking.rs` ranks `SellCommodity` — confirmed.
3. `crates/worldwake-ai/src/candidate_generation.rs` does not emit `SellCommodity` and already tests that it remains deferred — confirmed.
4. `crates/worldwake-ai/src/search.rs` rejects `SellCommodity` via `unsupported_goal()` — confirmed.
5. `specs/S04-merchant-selling-market-presence.md` says `SellCommodity` only becomes real alongside `SaleListing`, `staff_market`, listed-lot discovery, and removal of profile-inferred seller visibility — confirmed.
6. `MerchandiseProfile` currently represents enterprise intent, not concrete seller-side market presence — confirmed by S04 and by current belief-view seller discovery.

## Architecture Reassessment

The original implementation proposal is not more beneficial than the current architecture.

Why:

1. It would surface an unsatisfiable goal into planning while the planner still rejects `SellCommodity`.
2. It would strengthen the wrong abstraction: profile-based seller visibility instead of S04's concrete listed-lot model.
3. It would pull Phase 4+ seller-presence behavior forward into the FND-02 gate, despite `specs/IMPLEMENTATION-ORDER.md` placing S04 after E22.

The cleaner architecture is:

1. Keep `SellCommodity` deferred in Phase 2/FND-02.
2. Preserve the current invariant with explicit regression tests.
3. Implement seller-side selling only when S04 is taken as a full vertical slice.

## What to Change

### 1. Do Not Emit `SellCommodity` Yet

Do not add `emit_sell_goals()`, surplus detection, or partial seller-side logic to `candidate_generation.rs` or `enterprise.rs`.

### 2. Strengthen the Regression Coverage

Add or strengthen tests proving that `SellCommodity` stays deferred even when an agent has:

- `MerchandiseProfile`
- local stock
- a valid place / `home_market`
- remembered demand

This guards against reintroducing the stale assumption before S04 lands.

### 3. Preserve Existing Enterprise Behavior

Keep `RestockCommodity` and `MoveCargo` as the active enterprise goals in the current architecture.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify tests only)

## Out of Scope

- Do NOT add `SellCommodity` emission.
- Do NOT change `enterprise.rs`.
- Do NOT change planner support, action definitions, or trade handlers.
- Do NOT introduce `SaleListing`, `staff_market`, or listed-lot discovery outside S04.
- Do NOT modify `worldwake-core`, `worldwake-sim`, or `worldwake-systems` for this ticket.

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `SellCommodity` is not emitted for a merchant with `MerchandiseProfile`, local stock, `home_market`, and remembered demand.
2. Existing deferred-goal regression still passes.
3. Existing suite: `cargo test -p worldwake-ai`
4. Full suite: `cargo test --workspace`
5. Lint: `cargo clippy --workspace`

### Invariants

1. `SellCommodity` remains deferred until S04 provides concrete seller-side market presence.
2. `MerchandiseProfile` continues to represent enterprise intent, not active sale visibility.
3. Existing `RestockCommodity` and `MoveCargo` behavior remains unchanged.
4. No backward-compatibility shims or partial seller-side aliases are introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — strengthen `SellCommodity` deferral coverage for a merchant with stock, `home_market`, and demand memory.

### Commands

1. `cargo test -p worldwake-ai -- sell`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-13
- What actually changed:
  - Reassessed the ticket against current code and S04.
  - Corrected the ticket scope from "wire `SellCommodity` emission now" to "preserve `SellCommodity` deferral until S04".
  - Added a regression test in `crates/worldwake-ai/src/candidate_generation.rs` proving a merchant with stock, `home_market`, and remembered demand still does not emit `SellCommodity` before S04.
- Deviations from original plan:
  - No runtime code was added to `candidate_generation.rs` or `enterprise.rs`.
  - No surplus detection or partial seller-side behavior was introduced.
  - The ticket was completed by correcting the stale architectural assumption and strengthening the invariant coverage instead of implementing the originally proposed change.
- Verification results:
  - `cargo test -p worldwake-ai merchant_with_stock_and_demand_still_does_not_emit_sell_commodity_before_s04` passed.
  - `cargo test -p worldwake-ai still_deferred_goal_kinds_are_not_emitted` passed.
  - `cargo test -p worldwake-ai` passed.
  - `cargo clippy --workspace` passed.
  - `cargo test --workspace` passed.
