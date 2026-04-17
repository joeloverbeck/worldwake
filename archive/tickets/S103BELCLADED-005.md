# S103BELCLADED-005: Add time-aware entity summary invalidation for belief pruning

**Status**: REJECTED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — reassessment only
**Deps**: S101 (completed), archive/tickets/S103BELCLADED-002.md (restored optimization owner), archive/tickets/S103BELCLADED-004.md (completed)

## Problem

This ticket proposed a broad time-aware invalidation redesign for `known_entities`. Reassessment against the live ranking math showed that design is not the real owned problem. Under the live positive-threshold pruning contract, unchanged claim vectors keep the same winner until pruning actually removes a claim. The only surviving edge is the zero-threshold saturation case, and that is narrow enough to stay inside `S103BELCLADED-002` as a fallback to the old full-refresh path.

Keeping this ticket open would overstate the architecture change required and violate ticket fidelity.

## Assumption Reassessment (2026-04-14)

1. `derive_entity_summary` in `crates/worldwake-core/src/belief.rs` ranks per-aspect winners by `(effective_claim_confidence, acquired_tick, claim_id)` — verified.
2. `effective_claim_confidence` uses a uniform per-tick staleness slope for all claims. For the live runtime profiles that prune at `claim_confidence_threshold = 50`, unchanged claim membership preserves winner ordering until pruning removes a claim — verified by code inspection plus current profile usage across the repo.
3. The existing focused test `derive_entity_summary_applies_staleness_before_selecting_winner` proves staleness matters at a chosen tick, but it does not prove a time-only winner flip for an unchanged claim set under the live positive-threshold pruning contract.
4. A real edge exists only when `claim_confidence_threshold == 0`: saturated zero-confidence claims can remain stored and later tie on confidence, allowing static tie-breakers to change the winner without membership change.
5. `archive/tickets/S103BELCLADED-004.md` remains valid: it restored the semantic transport boundary needed for the real optimization in `S103BELCLADED-002`.
6. Mismatch + correction: this ticket should not own a broad cache redesign. The real implementation ticket is `S103BELCLADED-002`, with the zero-threshold edge handled locally inside that optimization.

## Architecture Check

1. Rejecting this ticket is cleaner than inventing a new invalidation subsystem for a problem the live positive-threshold contract does not have.
2. No backward-compatibility shims. The narrow zero-threshold edge belongs inside `S103BELCLADED-002`, not in a separate speculative architecture ticket.

## Verification Layers

1. Positive-threshold unchanged claim sets preserve winners without extra invalidation machinery -> reassessment proof in `worldwake-core`
2. Zero-threshold saturation edge is narrow and belongs to `S103BELCLADED-002` -> ticket-chain correction
3. Additional implementation work is not applicable here because this ticket is a reassessment-only rejection

## What to Change

### 1. Record the rejection precisely

Keep this ticket as a rejected reassessment record instead of an implementation target. The exact reason is that the broad time-aware invalidation redesign is not required by the live positive-threshold ranking contract.

### 2. Route the real work back to the restored optimization ticket

Use `S103BELCLADED-002` for the actual pruning optimization and keep the zero-threshold saturation edge inside that ticket as a local fallback to full refresh.

## Files to Touch

- `tickets/S103BELCLADED-005.md` (modify)
- `tickets/S103BELCLADED-002.md` (modify)
- `specs/S103-belief-claim-deduplication.md` (modify)

## Out of Scope

- Implementing the real pruning optimization
- Social observation deduplication (`S103BELCLADED-003`)
- Broad planner or candidate-generation changes

## Acceptance Criteria

### Tests That Must Pass

1. None — reassessment-only ticket; no owned code changes land here

### Invariants

1. The roadmap must no longer claim a broad time-aware invalidation redesign is needed
2. The actual pruning optimization is owned by `S103BELCLADED-002`

## Test Plan

### New/Modified Tests

1. None — documentation-only reassessment; implementation moved back to `S103BELCLADED-002`

### Commands

1. `cargo test -p worldwake-core --lib belief::tests::derive_entity_summary_applies_staleness_before_selecting_winner -- --exact`

## Outcome

Rejected on 2026-04-15.

- Reassessment against the live ranking math showed that the proposed broad time-aware invalidation redesign was not the real architectural need.
- The actual optimization owner was restored to `tickets/S103BELCLADED-002.md`.
- The narrow zero-threshold saturation edge was folded into `S103BELCLADED-002` as a local fallback requirement instead of keeping this separate architecture ticket open.

## Verification Result

- Passed `cargo test -p worldwake-core --lib belief::tests::derive_entity_summary_applies_staleness_before_selecting_winner -- --exact`
