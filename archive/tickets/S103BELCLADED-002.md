# S103BELCLADED-002: Reject changed-entity-only pruning refresh

**Status**: REJECTED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — reassessment only
**Deps**: S101 (completed), archive/tickets/S103BELCLADED-004.md (completed)

## Problem

This ticket originally proposed a local FND-12 optimization in `prune_decayed_beliefs`: after the retain pass, re-derive summaries only for entities whose claim vectors lost entries. Live verification showed that invariant is false. `known_entities` summaries are not a pure function of claim-vector membership alone; they also vary with `current_tick` because `derive_entity_summary` ranks winners by time-decayed effective confidence. An unchanged claim vector can therefore require a different winning summary at a later tick.

Shipping the original optimization would change planner-visible behavior and violate ticket fidelity.

## Assumption Reassessment (2026-04-14)

1. `prune_decayed_beliefs` in `crates/worldwake-core/src/belief.rs` still refreshes every entity summary after pruning — verified.
2. `derive_entity_summary` in `crates/worldwake-core/src/belief.rs` chooses winners using `effective_claim_confidence(claim, current_tick, policy)` — verified.
3. `effective_claim_confidence` decays with staleness, so summary winners can change as `current_tick` advances even when the claim vector is byte-for-byte unchanged — verified by code inspection and the existing focused test `derive_entity_summary_applies_staleness_before_selecting_winner`.
4. `archive/tickets/S103BELCLADED-004.md` successfully removed the mixed transport-path contradiction for the in-scope semantic fields, but it did not and could not remove time-based winner changes from the claim-ranking contract.
5. A local implementation of the original optimization was attempted and then reverted during reassessment. Broadened verification changed planner-visible behavior and exposed that the ticket's core invariant was false.
6. The exact abstraction boundary under audit is still `entity_claims` as stored evidence and `known_entities` as a time-sensitive derived summary cache. The rejected assumption was that unchanged evidence membership implied unchanged derived cache content.
7. Mismatch + correction: `S103BELCLADED-002` is not a lawful changed-entity-only optimization. The valid successor is a time-aware summary invalidation design owned by `S103BELCLADED-005`.

## Architecture Check

1. Rejecting the ticket is cleaner than weakening tests or adding ad hoc exceptions. The original optimization compresses causality, not just computation, because time is part of the summary-selection contract.
2. No backward-compatibility shims. The invalid optimization should remain unshipped rather than hidden behind a flag or partial heuristic.

## Verification Layers

1. Summary winners can change without membership changes -> existing focused unit proof: `derive_entity_summary_applies_staleness_before_selecting_winner`
2. The original optimization was not planner-neutral -> broadened `worldwake-ai` verification during reassessment
3. Reverting the attempted change restores the prior contract -> targeted rerun of existing focused prune coverage plus motivating golden verification
4. Additional change-layer mapping is not applicable because this ticket is a reassessment-only rejection

## What to Change

### 1. Record the rejection precisely

Keep this ticket as a rejected reassessment record instead of an implementation target. The exact reason is that `derive_entity_summary` is time-sensitive even when claim vectors are unchanged.

### 2. Route future optimization work to a lawful successor

Use `S103BELCLADED-005` for any future performance work in this area. That follow-up owns a time-aware summary invalidation or recompute-horizon design rather than the false changed-entity-only shortcut.

## Files to Touch

- `tickets/S103BELCLADED-002.md` (modify)
- `tickets/S103BELCLADED-005.md` (new)
- `specs/S103-belief-claim-deduplication.md` (modify)

## Out of Scope

- Implementing a replacement optimization
- Social observation deduplication (`S103BELCLADED-003`)
- Any new `known_entities` caching layer

## Acceptance Criteria

### Tests That Must Pass

1. None — reassessment-only ticket; no code changes remain

### Invariants

1. The roadmap must no longer claim that unchanged claim vectors imply unchanged summaries
2. Any future pruning optimization in this area must explicitly account for time-sensitive summary winner changes

## Test Plan

### New/Modified Tests

1. None — documentation-only reassessment; proof is the existing `derive_entity_summary_applies_staleness_before_selecting_winner` unit test plus the reverted failed implementation attempt recorded above

### Commands

1. `cargo test -p worldwake-core --lib belief::tests::derive_entity_summary_applies_staleness_before_selecting_winner -- --exact`

## Outcome

Rejected on 2026-04-15.

- Reassessment confirmed that the proposed changed-entity-only pruning refresh is invalid on the live contract because `derive_entity_summary` depends on `current_tick`, not just claim membership.
- A local implementation attempt was reverted after broadened verification showed that the optimization was not planner-neutral.
- Created `tickets/S103BELCLADED-005.md` as the lawful successor for a time-aware summary invalidation design.
- Updated `specs/S103-belief-claim-deduplication.md` so the active roadmap no longer claims the false `S103BELCLADED-004 -> S103BELCLADED-002` optimization sequence.

## Verification Result

- Passed `cargo test -p worldwake-core --lib belief::tests::derive_entity_summary_applies_staleness_before_selecting_winner -- --exact`
