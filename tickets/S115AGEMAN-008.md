# S115AGEMAN-008: locally bound purchase goal does not park to pending after seller departure

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI runtime / agenda lifecycle behavior.
**Deps**: [archive/tickets/S115AGEMAN-005](../archive/tickets/S115AGEMAN-005.md), [archive/tickets/S115AGEMAN-007](../archive/tickets/S115AGEMAN-007.md)

## Problem

At the live merchant-selling golden seam, a buyer can reach a real local `trade` binding against a merchant, but when the merchant leaves before trade execution the buyer's `AgendaState` remains `committed` instead of parking the `AcquireCommodity(Bread)` goal into `pending`. This blocks the originally drafted S115 purchase-revival golden and indicates a live contradiction between the intended agenda lifecycle and the current runtime behavior.

## Assumption Reassessment (2026-04-22)

1. The goal under test is `GoalKind::AcquireCommodity { commodity: Bread, purpose: SelfConsume }` on the merchant-selling substrate in `crates/worldwake-ai/tests/golden_merchant_selling.rs`.
2. Existing golden proof already reaches the pre-failure seam: `remote_branch_selection_reaches_local_trade_binding_before_merchant_departure` proves the buyer can arrive and hold a real local `trade` next step against the seller.
3. The first failing live boundary is not candidate generation. After the seller is moved away from that locally bound seam, the runtime continues to keep `AgentDecisionRuntime.agenda_state.committed` populated instead of demoting the goal into `pending`.
4. Focused repro command used during reassessment: `cargo test -p worldwake-ai --test golden_merchant_selling merchant_return_revives_pending_purchase_agenda_entry -- --exact`. Observed live snapshots across ticks 10-49: committed goal stayed `AcquireCommodity(Bread)` and `pending=0 suspended=0` throughout.
5. The likely shared boundary under audit is the handoff between local trade-step invalidation/start failure and agenda lifecycle parking: `agent_tick` execution / replanning / agenda-state transitions, not the observer surface and not the earlier remote-branch selection golden.
6. This is no longer a pure golden ticket. If the runtime contract is corrected, the purchase-revival golden can be reintroduced as proof; until then the golden remains blocked by the production contradiction.

## Architecture Check

1. The fix should land at the first runtime boundary that lawfully knows the local trade binding has been invalidated, rather than papering over the issue with observer-only logic or scenario scaffolding.
2. The end state should preserve `AgendaState` as the single authority for committed/pending/suspended lifecycle, with no duplicate state carrier or test-only fallback path.

## Verification Layers

1. Locally bound trade invalidation demotes the purchase goal from `committed` to `pending` -> focused `worldwake-ai` runtime/unit proof at the first failing `agent_tick` boundary.
2. The demoted pending entry carries the expected `RevivalTrigger::CounterpartyAvailable { counterparty, place }` -> focused runtime/unit proof.
3. Merchant return revives the pending entry and allows trade completion -> merchant-selling golden after the production seam is fixed.
4. Existing earlier seam (local trade binding before departure) remains green -> existing merchant-selling golden.

## What to Change

### 1. Identify the first failing runtime seam

Trace the locally bound `trade` step failure path after seller departure and determine where the runtime should convert the committed goal into `pending`.

### 2. Land the lifecycle fix

Update the relevant `agent_tick` / planning / failure-handling boundary so the invalidated committed purchase goal parks in `AgendaState.pending` with the correct counterparty revival trigger.

### 3. Add focused proof, then restore the golden

Add the strongest focused AI/runtime regression at the first failing seam. After that passes, add or restore a merchant-selling golden that proves pending -> revived -> trade-completed at the end-to-end seam.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/*` (expected modify — exact owner to be confirmed during implementation)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — restore honest purchase-revival golden once runtime seam is fixed)

## Out of Scope

- Observer rendering (landed in `S115AGEMAN-007`)
- Cargo `Suspended` observer/report proof

## Acceptance Criteria

### Tests That Must Pass

1. New focused `worldwake-ai` proof for committed->pending demotion after seller departure
2. Merchant-selling golden proving pending -> revived -> trade-completed after seller return
3. Existing seam check: `cargo test -p worldwake-ai --test golden_merchant_selling remote_branch_selection_reaches_local_trade_binding_before_merchant_departure -- --exact`

### Invariants

1. A locally invalidated committed purchase goal does not remain indefinitely committed when the seller has departed.
2. The parked pending goal carries a concrete `CounterpartyAvailable` revival trigger tied to the seller and market place.
3. Merchant return revives the pending goal through the real runtime lifecycle, not a test-only shortcut.

## Test Plan

### New/Modified Tests

1. Focused `worldwake-ai` runtime/unit test at the first failing `agent_tick` seam — proves committed -> pending demotion and revival-trigger shape.
2. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — proves pending -> revived -> completed trade after seller return.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai <focused-selector>`
3. `cargo test -p worldwake-ai --test golden_merchant_selling <restored-selector> -- --exact`
