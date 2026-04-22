# S114PLASTGUA-014: Re-author remote trade guard-breach golden after sale-stock routing fix

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `specs/S114-plan-step-guards.md`, `archive/tickets/S114PLASTGUA-010.md`, `archive/tickets/S114PLASTGUA-013.md`, `archive/tickets/S114PLASTGUA-011.md`

## Problem

S114 validation item 12 in `specs/S114-plan-step-guards.md` is still deferred. `archive/tickets/S114PLASTGUA-010.md` rejected the original golden because the live planner did not truthfully hold `Travel -> Trade` against remote displayed sale stock, and `archive/tickets/S114PLASTGUA-013.md` fixed that substrate gap. There is now no active ticket owning the truthful end-to-end golden that proves the remote `trade` step is guarded, invalidates when the seller departs before arrival, emits the expected mismatch payload, and replans through `Discrepancy::BeliefContradicted`.

## Assumption Reassessment (2026-04-22)

1. The remaining work is golden/E2E ownership, not production architecture. `archive/tickets/S114PLASTGUA-013.md` already landed the candidate-generation/search fix that makes remote listed sale stock route through `Travel -> Trade` instead of `Travel -> MoveCargo`.
2. The rejection record in `archive/tickets/S114PLASTGUA-010.md` remains the authoritative explanation for why the earlier golden draft was dishonest. This ticket must reuse that narrowed scenario premise instead of restoring the disproved setup.
3. Exact shared boundary under audit: buyer belief/perception setup in `crates/worldwake-ai/tests/golden_merchant_selling.rs`, planner selection of `AcquireCommodity { commodity: Bread, purpose: SelfConsume }`, runtime guard classification during the `trade` step, event-log mismatch payload emission, and AI discrepancy/replan handling.
4. Existing `golden_merchant_selling.rs` already contains nearby merchant-selling ownership, including `buyer_trades_against_listed_lot` and `seller_departure_invalidates_listing`. The live golden seam should extend that file rather than creating a new unrelated binary.
5. The truthful scenario must isolate the guarded remote-trade branch from lawful competing routes. In particular, the setup must not allow loose-cargo acquisition to displace the intended `trade` path, and it must seed the buyer's lawful remote beliefs so the planner can actually select the remote listed lot branch.
6. `archive/tickets/S114PLASTGUA-011.md` already established the expected mismatch-detail surface (`GuardInvalidator(TargetMoved)`) for downstream event emission. This ticket reuses that contract rather than redefining mismatch payload semantics.
7. If focused repro shows the autonomous golden still fails to hold the intended branch on the live harness, that is a premise failure for this ticket and must be corrected before implementation, not papered over with a weaker assertion.

## Architecture Check

1. Keeping this as a bounded golden in the existing merchant-selling suite is cleaner than reopening production code or scattering the proof across lower-layer tests, because the remaining gap is the end-to-end guarded-branch witness.
2. No backwards-compatibility shims or alternate planner paths are introduced; the golden should prove the live branch that now exists.

## Verification Layers

1. Buyer selects the remote seller-backed `Travel -> Trade` branch before departure -> decision trace and/or focused golden assertions
2. Seller departure invalidates the guarded `trade` step at arrival time -> runtime action/guard trace or event-log assertions
3. Mismatch payload is typed as `ExpectationKindTag::State` with `GuardInvalidator(TargetMoved)` -> focused event-log assertion
4. Failure is routed through `Discrepancy::BeliefContradicted` and replanning occurs within the expected window -> discrepancy memory / decision outcome assertions in the golden
5. Existing merchant-selling scenarios remain green -> focused `golden_merchant_selling` rerun

## What to Change

### 1. Add the deferred merchant-departure guard-breach golden

Extend `crates/worldwake-ai/tests/golden_merchant_selling.rs` with the truthful remote purchase scenario that now depends on the landed `-013` routing fix.

### 2. Prove the full guarded failure handoff

Assert the planned remote `trade` branch, seller departure before buyer arrival, guard invalidation payload, `Discrepancy::BeliefContradicted` routing, and bounded replan timing.

## Files to Touch

- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — add the deferred remote trade guard-breach golden)
- `specs/S114-plan-step-guards.md` (modify — replace the generic deferred note with the landed owner path if needed during closeout)

## Out of Scope

- Further production fixes to remote commodity routing or planner snapshot carriage
- New guard kinds, mismatch payload fields, or discrepancy taxonomy changes
- Merchant enterprise behavior beyond the authored guarded-departure scenario

## Acceptance Criteria

### Tests That Must Pass

1. A golden scenario proves a buyer plans remote `Travel -> Trade`, the seller departs before arrival, and the guard invalidates the `trade` step on arrival.
2. The same golden proves `ExpectationMismatch` includes `ExpectationKindTag::State` plus `GuardInvalidator(TargetMoved)`.
3. The same golden proves `Discrepancy::BeliefContradicted` is recorded and the agent replans within 2 ticks.
4. Existing merchant-selling golden coverage stays green.

### Invariants

1. The golden proves the truthful guarded remote-trade branch, not a competing loose-cargo or no-candidate path.
2. The failure is observed at the guard/revalidation boundary, not deferred to a handler-only rejection path.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — add the deferred remote seller-departure guard-breach scenario now that `Travel -> Trade` is truthful.
2. `None` — no production code changes are expected; existing focused planner-routing coverage from `archive/tickets/S114PLASTGUA-013.md` remains the substrate proof.

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling seller_departure_invalidates_trade_plan_via_guard_breach -- --exact --nocapture`
2. `cargo test -p worldwake-ai --test golden_merchant_selling buyer_trades_against_listed_lot -- --exact`
3. `cargo test -p worldwake-ai --lib search::tests::search_returns_travel_then_trade_barrier_for_remote_listed_sale_lot_without_custody_detail -- --exact`
4. `cargo test -p worldwake-ai --test golden_merchant_selling`
