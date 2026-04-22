# S115AGEMAN-009: seller return should restore lawful market-selling state after buyer-side agenda revival

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — trade/market presence and AI-integrated merchant lifecycle behavior.
**Deps**: [archive/tickets/S115AGEMAN-008](../archive/tickets/S115AGEMAN-008.md), [archive/tickets/S115AGEMAN-005](../archive/tickets/S115AGEMAN-005.md), [archive/tickets/S115AGEMAN-007](../archive/tickets/S115AGEMAN-007.md)

## Problem

After `S115AGEMAN-008`, a buyer whose locally bound purchase goal was parked to `pending` now revives back into committed/current-plan state when the seller returns. But the broader drafted story still does not complete: the returning seller does not re-enter a lawful market-selling state that lets the revived buyer actually finish the trade against a live listing.

## Assumption Reassessment (2026-04-22)

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` now proves the buyer-side boundary truthfully with `merchant_return_revives_pending_purchase_agenda_entry`, and that test intentionally stops at revived `committed/current_plan` rather than trade completion.
2. The earlier pre-failure seam is still covered by `remote_branch_selection_reaches_local_trade_binding_before_merchant_departure`, which proves the buyer can reach a concrete local `trade` binding before seller departure.
3. The shared boundary under audit for this follow-up is no longer buyer agenda lifecycle. That boundary now passes in `agent_tick/planning.rs`. The remaining live question is seller-side market presence / listing readiness after authoritative departure and return.
4. The intended invariant is narrower than “seller is back at the same place.” A seller return must also restore whatever concrete state the buyer-side `trade` step lawfully depends on: seller presence, market/facility readiness, and sale-visible stock/listing continuity or lawful restaging.
5. This ticket is mixed-layer: authoritative seller/facility/listing state in the merchant-selling substrate must line up with the AI/runtime path that resumes the revived buyer plan.
6. Reassessment from `S115AGEMAN-008` classified this as a separate bug, not a required consequence of the buyer-side pending-revival fix, so it needs its own focused ticket and truthful proof.

## Architecture Check

1. The fix should restore the seller-side authoritative state transition that makes resumed trade lawful, rather than adding buyer-side special cases or test-only reseeding.
2. No compatibility shim should preserve both “seller returned but not market-ready” and “seller returned and lawfully sellable” as parallel contracts.

## Verification Layers

1. Seller departure/return preserves or lawfully restores the merchant-selling authoritative substrate needed for trade -> authoritative world-state and focused merchant-selling golden proof.
2. A revived buyer-side purchase plan can actually complete the trade after seller return -> merchant-selling golden with real runtime lifecycle, not manual state injection after return.
3. Buyer agenda revival remains intact while the seller-side fix lands -> existing `merchant_return_revives_pending_purchase_agenda_entry` golden or a tightened successor proof.
4. If traces are needed to explain why trade still does not restart, capture the strongest lower-layer proof instead of inferring from missing bread transfer alone.

## What to Change

### 1. Reassess the seller-side return contract

Trace what concrete merchant-selling state is missing after seller departure and return: listing continuity, facility/display readiness, staff-market re-entry, or another authoritative prerequisite.

### 2. Restore the lawful seller-side path

Implement the narrowest production fix so a returned seller can once again support the buyer’s revived local `trade` path without observer-only or test-only scaffolding.

### 3. Extend the golden boundary truthfully

Add or revise merchant-selling golden coverage so the full resumed story reaches actual trade completion only if live code now supports that contract.

## Files to Touch

- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify)
- `crates/worldwake-systems/src/*` (expected modify if authoritative market/listing lifecycle owns the missing seam)
- `crates/worldwake-ai/src/*` (modify only if reassessment proves the remaining gap is still AI/runtime-owned)

## Out of Scope

- Reopening buyer-side pending parking / revival logic already landed in `S115AGEMAN-008`
- Observer-only reporting tweaks

## Acceptance Criteria

### Tests That Must Pass

1. Merchant-selling golden proving revived buyer-side trade can complete after seller return, if that is the truthful restored contract.
2. Existing buyer-side revival seam remains green.
3. Existing suite: `cargo test -p worldwake-ai --test golden_merchant_selling`

### Invariants

1. Seller return does not leave the buyer’s revived trade path stranded on a non-lawful merchant-selling substrate.
2. The resumed trade completion path uses real seller/facility/listing state, not ad hoc reseeding or compatibility shims.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — extend the revived-return scenario to prove actual completion only if the seller-side contract is restored.
2. Focused authoritative or runtime test at the first failing seller-side seam once reassessment identifies it.

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling merchant_return_revives_pending_purchase_agenda_entry -- --exact`
2. `cargo test -p worldwake-ai --test golden_merchant_selling`
3. `cargo test -p worldwake-ai`
