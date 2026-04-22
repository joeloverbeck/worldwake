# S114PLASTGUA-015: Pair the truthful merchant trade-step golden seam with the existing focused guard-breach runtime proof

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — truthful proof-surface reassessment and coverage only
**Deps**: `specs/S114-plan-step-guards.md`, `archive/tickets/S114PLASTGUA-014.md`, `archive/tickets/S114PLASTGUA-011.md`

## Problem

After `archive/tickets/S114PLASTGUA-014.md` fixed the last remaining seller-backed container-detail routing bug, the originally drafted autonomous end-to-end golden is still not truthful on the live branch. The buyer can select the remote `Travel -> Trade` branch, but arrival remains a progress barrier; once the seller departs, the fully autonomous runtime does not hold a stale local trade step long enough to emit the expected guard-breach `ExpectationMismatch` payload. The observed live path replans through `BlockingFact::TooExpensive` instead. S114 validation item 12 still needs a truthful proof owner for the guard-breach mismatch contract, but that owner can no longer be a single golden that both holds the remote branch stable and emits the same AI pre-enqueue mismatch payload.

## Assumption Reassessment (2026-04-22)

1. `archive/tickets/S114PLASTGUA-014.md` now owns the final production routing fix required before any truthful guard-breach proof can exist. This ticket must build on that landed search behavior instead of reopening `MoveCargo` versus `Trade` routing.
2. Exact live `GoalKind` under test remains `AcquireCommodity { commodity: Bread, purpose: SelfConsume }`, but the immediate golden proof boundary is no longer the initial remote travel plan alone. The strongest honest golden seam is: remote branch selection followed by a concrete local `trade` next step before seller departure.
3. First live failure boundary under reassessment remains unchanged: fully autonomous post-arrival replanning after seller departure records `BlockingFact::TooExpensive`, not `ExpectationMismatch`. That means the drafted autonomous stale-window is false on the live branch.
4. `archive/tickets/S114PLASTGUA-011.md` already provides the focused runtime proof that a guarded trade step can emit `ExpectationMismatch` with `GuardInvalidator(TargetMoved)` and record `Discrepancy::BeliefContradicted`. Reusing that lower-layer contract is cleaner than trying to force the same event through a golden harness seam that does not own AI pre-enqueue validation.
5. `docs/golden-e2e-testing.md` prefers the earliest causal golden boundary that proves the scenario-authored reason for success. On the live branch, that boundary is decision-trace proof that the buyer selected the remote seller-backed branch and later reached a local guarded `trade` step, not a scripted external request.
6. A scripted hybrid `RequestAction` seam would route through sim request resolution, not the AI execution helper that emits `EventTag::ExpectationMismatch` on guard breach. Treating that scripted request as equivalent to the AI pre-enqueue mismatch path would overstate the golden contract.
7. Strongest honest proof set is therefore paired: golden merchant-selling coverage proves the remote branch plus local trade-step binding, while the existing focused `agent_tick::tests::revalidation_guard_breach_emits_expectation_mismatch_before_enqueue` proof continues to own the mismatch event payload and discrepancy-memory aftermath.
8. Scenario isolation remains required. The golden must remove unrelated lawful affordances that would otherwise let the buyer switch to another food source or another seller before the owned branch-selection / local-binding seam is exercised.

## Architecture Check

1. Splitting the proof across the earliest honest golden boundary and the already-existing focused runtime boundary is cleaner than forcing more production changes or a misleading scripted-request story just to preserve an earlier stale narrative that the live runtime no longer supports.
2. No backward-compatibility shims are expected. The goal is to prove the existing live guard-breach contract honestly, not to preserve a superseded autonomous story.

## Verification Layers

1. AI selects the remote seller-backed branch before arrival -> decision trace in `golden_merchant_selling.rs`
2. AI reaches a truthful local guarded trade-step binding -> decision trace / selected-plan proof at the local trade-step seam
3. Guarded trade-step invalidation emits `ExpectationMismatch(GuardInvalidator(TargetMoved))` -> existing focused runtime assertion in `agent_tick::tests::revalidation_guard_breach_emits_expectation_mismatch_before_enqueue`
4. `DiscrepancyMemory` records `Discrepancy::BeliefContradicted` -> existing focused runtime assertion in `agent_tick::tests::revalidation_guard_breach_emits_expectation_mismatch_before_enqueue`
5. The ticket must not claim one golden proves all layers when the live mismatch event still belongs to the lower execution seam

## What to Change

### 1. Reassess the live stale-window and choose the honest proof seam

Use `crates/worldwake-ai/tests/golden_merchant_selling.rs` only for the earliest honest merchant-selling seam: remote seller-backed branch selection plus the later local guarded `trade` binding. Reuse the existing focused `agent_tick` proof for the mismatch event and discrepancy routing instead of trying to recreate that event through a scripted request.

### 2. Land the truthful paired proof set

Add the strongest honest regression for:

- remote AI branch selection
- local trade-step binding before departure
- the already-owned focused runtime mismatch seam:
  `ExpectationMismatch` + `GuardInvalidator(TargetMoved)` + `Discrepancy::BeliefContradicted`

## Files to Touch

- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — remote branch selection plus local trade-step binding golden proof)
- `tickets/S114PLASTGUA-015.md` (modify — reassessment and closeout)
- `specs/S114-plan-step-guards.md` (modify — record the paired proof surface for validation item 12)

## Out of Scope

- More search/candidate-generation production fixes already owned by `archive/tickets/S114PLASTGUA-014.md`
- New guard-template authoring, payload-schema changes, or new runtime mismatch producers
- General replanning-policy redesign outside the owned proof seam

## Acceptance Criteria

### Tests That Must Pass

1. A truthful regression proves the remote seller-backed branch and the later guarded trade-step binding at the live seam
2. Existing focused runtime coverage proves `ExpectationMismatch` with `GuardInvalidator(TargetMoved)` for the guarded `trade` step
3. Existing focused runtime coverage proves `Discrepancy::BeliefContradicted` routing at the same honest execution boundary

### Invariants

1. The ticket must not claim a fully autonomous stale-window or a scripted-request mismatch event if the live runtime still disproves those seams
2. The final proof must use the strongest honest boundary available, even when that means a paired golden-plus-focused-runtime proof instead of one end-to-end test

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — prove remote branch selection and the later local guarded `trade` binding before seller departure
2. `None — mismatch event and discrepancy routing remain owned by the existing focused runtime test from S114PLASTGUA-011`

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling -- --list`
2. `cargo test -p worldwake-ai --test golden_merchant_selling remote_branch_selection_reaches_local_trade_binding_before_merchant_departure -- --exact`
3. `cargo test -p worldwake-ai --lib agent_tick::tests::revalidation_guard_breach_emits_expectation_mismatch_before_enqueue -- --exact`
4. `cargo test -p worldwake-ai --test golden_merchant_selling`

## Outcome

Completed on 2026-04-22.

- Added `remote_branch_selection_reaches_local_trade_binding_before_merchant_departure` to `crates/worldwake-ai/tests/golden_merchant_selling.rs`. The new golden proves the earliest honest merchant-selling boundary on the live branch: the buyer first selects a remote seller-backed `Travel -> Trade` path, then after arrival reaches a concrete local `trade` next step bound to that seller before any departure-induced invalidation.
- Reassessed the drafted hybrid seam truthfully and kept the mismatch event on its real owner. `EventTag::ExpectationMismatch` for the guarded `trade` step remains owned by the focused AI execution proof from `archive/tickets/S114PLASTGUA-011.md`, because a scripted external `RequestAction` would route through sim request resolution instead of the AI pre-enqueue validation branch that emits the mismatch payload.
- Updated `specs/S114-plan-step-guards.md` validation item 12 to record the paired proof surface instead of a single autonomous or scripted-hybrid golden claim.

## Deviations

- Draft ticket 015 was authored around a possible hybrid golden that would carry the local trade-step binding into the same mismatch event seam. Live reassessment showed that scripted requests are not equivalent to the AI pre-enqueue execution path, so the ticket narrowed to the strongest honest paired proof set: golden branch-selection plus local binding here, focused runtime mismatch proof reused from `S114PLASTGUA-011`.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_merchant_selling -- --list`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling remote_branch_selection_reaches_local_trade_binding_before_merchant_departure -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::revalidation_guard_breach_emits_expectation_mismatch_before_enqueue -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling`
