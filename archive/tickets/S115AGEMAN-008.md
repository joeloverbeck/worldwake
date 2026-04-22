# S115AGEMAN-008: locally bound purchase goal now parks to pending and revives after seller return

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI runtime / agenda lifecycle behavior.
**Deps**: [archive/tickets/S115AGEMAN-005](./S115AGEMAN-005.md), [archive/tickets/S115AGEMAN-007](./S115AGEMAN-007.md)

## Problem

At the live merchant-selling golden seam, a buyer can reach a real local `trade` binding against a merchant, but when the merchant leaves before trade execution the buyer's `AgendaState` remains `committed` instead of parking the `AcquireCommodity(Bread)` goal into `pending`. This blocks the originally drafted S115 purchase-revival golden and indicates a live contradiction between the intended agenda lifecycle and the current runtime behavior.

## Assumption Reassessment (2026-04-22)

1. The goal under test is `GoalKind::AcquireCommodity { commodity: Bread, purpose: SelfConsume }` on the merchant-selling substrate in `crates/worldwake-ai/tests/golden_merchant_selling.rs`.
2. Existing golden proof already reaches the pre-failure seam: `remote_branch_selection_reaches_local_trade_binding_before_merchant_departure` proves the buyer can arrive and hold a real local `trade` next step against the seller.
3. The first failing live boundary is not candidate generation. After the seller is moved away from that locally bound seam, the runtime continues to keep `AgentDecisionRuntime.agenda_state.committed` populated instead of demoting the goal into `pending`.
4. Focused repro command used during reassessment: `cargo test -p worldwake-ai --test golden_merchant_selling merchant_return_revives_pending_purchase_agenda_entry -- --exact`. Initial live snapshots across ticks 10-49 showed the committed goal staying `AcquireCommodity(Bread)` with `pending=0 suspended=0` throughout.
5. The likely shared boundary under audit is the handoff between local trade-step invalidation/start failure and agenda lifecycle parking: `agent_tick` execution / replanning / agenda-state transitions, not the observer surface and not the earlier remote-branch selection golden.
6. Reassessment during implementation exposed a second, separate seam: once the buyer-side agenda entry revives, the later seller-market reactivation / trade-completion story is not the same bug. The truthful current ticket slice is buyer-side `pending -> committed/current_plan` revival; the broader seller-side completion contract is split to follow-up ticket `S115AGEMAN-009`.

## Architecture Check

1. The fix should land at the first runtime boundary that lawfully knows the local trade binding has been invalidated, rather than papering over the issue with observer-only logic or scenario scaffolding.
2. The end state should preserve `AgendaState` as the single authority for committed/pending/suspended lifecycle, with no duplicate state carrier or test-only fallback path.

## Verification Layers

1. Locally bound trade invalidation demotes the purchase goal from `committed` to `pending` -> focused `worldwake-ai` runtime/unit proof at the first failing `agent_tick` boundary.
2. The demoted pending entry carries the expected `RevivalTrigger::CounterpartyAvailable { counterparty, place }` -> focused runtime/unit proof.
3. Merchant return revives the pending entry back into committed/current-plan state -> merchant-selling golden after the production seam is fixed.
4. Existing earlier seam (local trade binding before departure) remains green -> existing merchant-selling golden.

## What to Change

### 1. Identify the first failing runtime seam

Trace the locally bound `trade` step failure path after seller departure and determine where the runtime should convert the committed goal into `pending`.

### 2. Land the lifecycle fix

Update the relevant `agent_tick` / planning / failure-handling boundary so the invalidated committed purchase goal parks in `AgendaState.pending` with the correct counterparty revival trigger.

### 3. Add focused proof, then restore the truthful golden

Add the strongest focused AI/runtime regressions at the parking and repair-resumption seams. After that passes, restore a merchant-selling golden that proves `pending -> revived/current-plan` at the honest buyer-side seam.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/*` (expected modify — exact owner to be confirmed during implementation)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — restore honest purchase-revival golden once runtime seam is fixed)

## Out of Scope

- Observer rendering (landed in `S115AGEMAN-007`)
- Cargo `Suspended` observer/report proof

## Acceptance Criteria

### Tests That Must Pass

1. New focused `worldwake-ai` proof for committed->pending demotion after seller departure
2. Merchant-selling golden proving pending -> revived/current-plan after seller return
3. Existing seam check: `cargo test -p worldwake-ai --test golden_merchant_selling remote_branch_selection_reaches_local_trade_binding_before_merchant_departure -- --exact`

### Invariants

1. A locally invalidated committed purchase goal does not remain indefinitely committed when the seller has departed.
2. The parked pending goal carries a concrete `CounterpartyAvailable` revival trigger tied to the seller and market place.
3. Merchant return revives the pending goal back into committed/current-plan state through the real runtime lifecycle.

## Test Plan

### New/Modified Tests

1. Focused `worldwake-ai` runtime/unit test at the first failing `agent_tick` seam — proves committed -> pending demotion and revival-trigger shape.
2. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — proves pending -> revived/current-plan after seller return at the buyer-side agenda seam.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai <focused-selector>`
3. `cargo test -p worldwake-ai --test golden_merchant_selling <restored-selector> -- --exact`

## Outcome

Completed on 2026-04-22.

- `agent_tick/planning.rs` now parks a locally invalidated committed trade purchase goal into `AgendaState.pending` with `RevivalTrigger::CounterpartyAvailable`.
- The same planning seam now resumes the stored failed trade plan when that counterparty-availability trigger revives the goal, so the buyer-side runtime returns to committed/current-plan state instead of looping in `pending`.
- Restored merchant-selling golden coverage at the truthful revived buyer-side seam.

## Deviations

- The drafted broader `pending -> revived -> trade-completed` story was too strong for this ticket. Live proof showed the buyer-side agenda lifecycle and the later seller-market reactivation / completion behavior are separable concerns.
- Follow-up ticket `S115AGEMAN-009` now owns the remaining seller-side market-restaging / completion gap.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::pending_trigger_from_failed_plan_uses_trade_counterparty_and_place -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::clear_current_plan_parks_committed_trade_goal_into_pending_repair -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::resume_pending_repair_plan_restores_failed_trade_plan_when_counterparty_trigger_revives -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling remote_branch_selection_reaches_local_trade_binding_before_merchant_departure -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling merchant_return_revives_pending_purchase_agenda_entry -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling`
