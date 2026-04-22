# S114PLASTGUA-015: Re-author merchant-departure guard-breach proof at the truthful hybrid/local trade-step seam

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None expected — proof-seam reassessment first
**Deps**: `specs/S114-plan-step-guards.md`, `archive/tickets/S114PLASTGUA-014.md`, `archive/tickets/S114PLASTGUA-011.md`

## Problem

After `archive/tickets/S114PLASTGUA-014.md` fixed the last remaining seller-backed container-detail routing bug, the originally drafted autonomous end-to-end golden is still not truthful on the live branch. The buyer can select the remote `Travel -> Trade` branch, but arrival remains a progress barrier; once the seller departs, the fully autonomous runtime does not hold a stale local trade step long enough to emit the expected guard-breach `ExpectationMismatch` payload. The observed live path replans through `BlockingFact::TooExpensive` instead. S114 validation item 12 still needs a truthful proof owner for the guard-breach mismatch contract, but that owner must now use the honest hybrid/local trade-step seam instead of the disproved “single autonomous remote plan survives until enqueue” narrative.

## Assumption Reassessment (2026-04-22)

1. `archive/tickets/S114PLASTGUA-014.md` now owns the final production routing fix required before any truthful guard-breach proof can exist. This ticket must build on that landed search behavior instead of reopening `MoveCargo` versus `Trade` routing.
2. Exact live `GoalKind` under test remains `AcquireCommodity { commodity: Bread, purpose: SelfConsume }`, but the immediate proof boundary is no longer the initial remote travel plan alone. The actionable local seam is the post-arrival trade-step selection and the next-start rejection window.
3. First live failure boundary under reassessment: fully autonomous post-arrival replanning after seller departure records `BlockingFact::TooExpensive`, not `ExpectationMismatch`. That means the drafted autonomous stale-window is false on the live branch.
4. `archive/tickets/S114PLASTGUA-011.md` still provides the focused runtime proof that a guarded trade step can emit `ExpectationMismatch` with `GuardInvalidator(TargetMoved)` and record `Discrepancy::BeliefContradicted`. This ticket should reuse that lower-layer contract rather than redefining it.
5. The strongest honest remaining golden seam is likely hybrid per `docs/golden-e2e-testing.md`: prove the AI selected the remote branch, advance to the first truthful local trade-step selection, carry that exact binding through the narrowest lawful scripted request or equivalent harness seam, then prove the guard-breach runtime aftermath.
6. If implementation shows that even the hybrid golden cannot lawfully exercise the same `ExpectationMismatch` / discrepancy surface without dropping to a lower-layer harness, this ticket must narrow again instead of overstating an end-to-end proof.
7. Scenario isolation remains required. The test must remove unrelated lawful affordances that would otherwise let the buyer switch to another food source or another seller before the owned guard-breach seam is exercised.

## Architecture Check

1. Re-authoring the proof at the truthful hybrid/local trade-step seam is cleaner than forcing more production changes just to preserve an earlier stale narrative that the live runtime no longer supports.
2. No backward-compatibility shims are expected. The goal is to prove the existing live guard-breach contract honestly, not to preserve a superseded autonomous story.

## Verification Layers

1. AI selects the remote seller-backed branch before arrival -> decision trace in `golden_merchant_selling.rs`
2. AI reaches a truthful local guarded trade-step binding -> decision trace / selected-plan proof at the local trade-step seam
3. Seller departure invalidates that guarded trade step with `ExpectationMismatch(GuardInvalidator(TargetMoved))` -> event-log assertion at the truthful runtime boundary
4. `DiscrepancyMemory` records `Discrepancy::BeliefContradicted` and replanning follows -> persisted discrepancy memory plus later decision trace or focused hybrid-runtime assertion
5. If the golden still cannot isolate the event-emission seam truthfully, update the ticket to the strongest lower-layer proof instead of forcing a false end-to-end claim

## What to Change

### 1. Reassess the live stale-window and choose the honest proof seam

Determine whether the remaining contract can be proved in `crates/worldwake-ai/tests/golden_merchant_selling.rs` through a hybrid request seam, or whether the ticket must narrow to a lower-layer runtime proof plus a lighter golden branch-selection assertion.

### 2. Land the truthful guard-breach proof

Add the strongest honest regression for:

- remote AI branch selection
- local trade-step binding before departure
- seller departure before the selected trade step can lawfully enqueue
- `ExpectationMismatch` + `GuardInvalidator(TargetMoved)` + `Discrepancy::BeliefContradicted`

## Files to Touch

- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — truthful hybrid/local trade-step guard-breach proof)
- `tickets/S114PLASTGUA-015.md` (modify — reassessment and closeout)
- `specs/S114-plan-step-guards.md` (modify if the final proof seam narrows again)

## Out of Scope

- More search/candidate-generation production fixes already owned by `archive/tickets/S114PLASTGUA-014.md`
- New guard-template authoring or payload-schema changes
- General replanning-policy redesign outside the owned proof seam

## Acceptance Criteria

### Tests That Must Pass

1. A truthful regression proves the remote seller-backed branch and the later guarded trade-step binding at the live seam
2. The same regression or a paired focused proof proves `ExpectationMismatch` with `GuardInvalidator(TargetMoved)` for the departed seller
3. The same proof set demonstrates `Discrepancy::BeliefContradicted` routing and replanning at the honest boundary

### Invariants

1. The ticket must not claim a fully autonomous stale-window if the live runtime still disproves it
2. The final proof must use the strongest honest boundary available, even if that boundary is hybrid rather than purely autonomous

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — re-author the merchant-departure proof at the truthful hybrid/local trade-step seam
2. `None — if reassessment proves a lower-layer runtime proof is the strongest honest boundary, update this ticket and use the existing focused agent_tick coverage instead of forcing a new golden`

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling -- --list`
2. `cargo test -p worldwake-ai --test golden_merchant_selling <exact_selector_after_reassessment> -- --exact --nocapture`
3. `cargo test -p worldwake-ai --lib agent_tick::tests::revalidation_guard_breach_emits_expectation_mismatch_before_enqueue -- --exact`
