# S01PROOUTOWNCLA-012: Fix golden merchant restock trade test regression

**Status**: ✅ COMPLETED (resolved by prior work)
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Likely — merchant restock planner behavior under actor-owned output
**Deps**: S01PROOUTOWNCLA-004 (introduced the regression), S01PROOUTOWNCLA-011 (related planner fixes)

## Problem

`golden_merchant_restock_return_stock` and its replay test have been failing since S01PROOUTOWNCLA-004 (harvest commit ownership). The same root cause pattern as -011: the GOAP planner's behavior changes when production output becomes actor-owned instead of unowned, likely affecting how merchant restock agents plan harvest → pick_up → trade sequences.

## Assumption Reassessment (2026-03-16)

1. Tests pass at S01PROOUTOWNCLA-009a (c00dcd9) — confirmed
2. Tests fail at S01PROOUTOWNCLA-004 (1f96e43) — confirmed
3. Tests remain broken at HEAD — confirmed
4. The -011 CONSUME_OPS and search preference fixes did not resolve these failures — confirmed

## Architecture Check

1. The -011 fixes addressed `ConsumeOwnedCommodity` goal planning. The trade test likely involves `RestockCommodity` or `SellCommodity` goals, which may have similar GOAP search issues when production output gains ownership.
2. Investigation should follow the same pattern as -011: trace candidate generation → search → terminal ordering for the affected goal kinds.
3. Merchant agents with `PerceptionProfile` may need to be verified — the same perception gap that affected production tests may apply.

## Files to Touch

- `crates/worldwake-ai/tests/golden_trade.rs` (likely test setup changes)
- `crates/worldwake-ai/src/goal_model.rs` (possible op list or barrier adjustments for restock/sell goals)

## Test Plan

### Commands

1. `cargo test -p worldwake-ai --test golden_trade golden_merchant_restock_return_stock`
2. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-16
- **What changed**: No code changes required. The regression was resolved by prior tickets S01PROOUTOWNCLA-011 (CONSUME_OPS narrowing, barrier fix, GoalSatisfied search preference) and S01PROOUTOWNCLA-008 (belief affordance filtering). Both `golden_merchant_restock_return_stock` and its replay test pass at HEAD.
- **Deviations**: Ticket assumed the regression persisted; it had already been fixed by the time this ticket was reached in the implementation order.
- **Verification**: `cargo test -p worldwake-ai --test golden_trade golden_merchant_restock_return_stock` — 2/2 passed. `cargo test --workspace` — all passed.
