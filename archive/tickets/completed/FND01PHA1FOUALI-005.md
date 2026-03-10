# FND01PHA1FOUALI-005: Clarify Loyalty Mutation Boundaries and Close Delta Regression Gaps

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — documentation and verification only, no behavioral changes
**Deps**: None (independent)

## Problem

`LoyalTo.strength: Permille` is a scalar disposition. Scalars are allowed in Worldwake only when they remain grounded in concrete state and do not become hidden script switches. The loyalty mutation path is already architected around canonical weighted-relation deltas, but the current ticket overstated the missing test coverage and the allowed mutation boundary. We need to document the real boundary and add the remaining focused regression coverage without introducing loyalty-specific plumbing.

## Assumption Reassessment (2026-03-10)

1. `World::set_loyalty()` and `World::clear_loyalty()` are confirmed internal mutation helpers in `crates/worldwake-core/src/world/social.rs`. They delegate to the generic weighted-relation tables and should stay generic.
2. `WorldTxn::set_loyalty()` and `WorldTxn::clear_loyalty()` in `crates/worldwake-core/src/world_txn.rs` already event-source loyalty mutations through `push_weighted_relation_delta(...)`.
3. `push_weighted_relation_delta(...)` already handles the correct canonical semantics for weighted relation changes:
   - unchanged value => no delta
   - add => `RelationDelta::Added`
   - clear => `RelationDelta::Removed`
   - strength update => `Removed(old)` then `Added(new)`
4. Existing tests already cover part of the add-path:
   - `world_txn.rs::social_and_ownership_wrappers_record_relation_deltas` verifies that `txn.set_loyalty(...)` produces a loyalty `RelationDelta::Added`.
   - `world.rs::loyalty_and_hostility_queries_stay_bidirectional_and_strength_aware` verifies direct world-state behavior for loyalty set/update/clear.
5. The real test gaps are narrower:
   - no focused regression test for `txn.clear_loyalty(...)`
   - no focused regression test for strength updates emitting `Removed(old)` + `Added(new)`
   - no commit-level test proving the emitted event record preserves loyalty relation deltas after `WorldTxn::commit(...)`
6. The earlier wording "all runtime changes MUST flow through `WorldTxn`" is too absolute for the current architecture. `World` mutation helpers are still used legitimately for bootstrap/internal world maintenance and direct world-level tests. The durable boundary is:
   - event-sourced simulation/system mutations should go through `WorldTxn`
   - low-level `World` helpers remain internal primitives and must not gain alternate loyalty-specific mutation paths

## Architecture Check

1. The current architecture is already the right shape: loyalty uses the same generic weighted-relation machinery as other canonical relations, and `WorldTxn` is the event-sourced wrapper over those primitives.
2. Introducing loyalty-specific delta plumbing would make the design worse, not better. This ticket should preserve the generic helper and document intent around it.
3. The right improvement is to make the mutation boundary explicit in docs and close the remaining regression gaps around clear/update/commit behavior.
4. This ticket does not implement belief-based loyalty reasoning. It only prevents future regressions that would let loyalty mutations drift away from canonical event-log semantics.

## What to Change

### 1. Add doc-comments to `World::set_loyalty()` and `World::clear_loyalty()`

In `crates/worldwake-core/src/world/social.rs`, add succinct doc-comments that make these points explicit:

- these are internal low-level world mutation helpers
- event-sourced simulation mutations belong at the `WorldTxn` layer
- loyalty is a scalar relation input, not a scripting threshold or alternate decision pipeline

Do not claim that `World` can never be used directly; that would contradict existing legitimate internal/bootstrap usage.

### 2. Add doc-comments to `WorldTxn::set_loyalty()` and `WorldTxn::clear_loyalty()`

In `crates/worldwake-core/src/world_txn.rs`, document that these are the public event-sourced mutation entry points for loyalty changes and that they preserve canonical `RelationDelta` semantics via the shared weighted-relation helper.

### 3. Add focused loyalty regression tests in `world_txn.rs`

Add/strengthen focused tests that verify:

1. `txn.clear_loyalty(...)` records a loyalty `RelationDelta::Removed`
2. updating loyalty strength records `RelationDelta::Removed(old)` followed by `RelationDelta::Added(new)`
3. after `WorldTxn::commit(...)`, the emitted `EventRecord` still contains the expected loyalty relation deltas

These tests should complement the existing broad wrapper coverage rather than duplicating it verbatim.

## Files to Touch

- `crates/worldwake-core/src/world/social.rs` (modify — add doc-comments)
- `crates/worldwake-core/src/world_txn.rs` (modify — add doc-comments + focused regression tests)

## Out of Scope

- Do NOT change loyalty mutation behavior.
- Do NOT introduce loyalty-specific delta helpers or bypass the shared weighted-relation mutation path.
- Do NOT modify `Permille`, `RelationDelta`, or `RelationTables`.
- Do NOT add value validation logic for loyalty strength.
- Do NOT implement belief-based loyalty evaluation or new AI behavior.

## Acceptance Criteria

### Tests That Must Pass

1. `World::set_loyalty()` and `World::clear_loyalty()` clearly document their role as internal mutation helpers and point event-sourced callers toward `WorldTxn`.
2. `WorldTxn::set_loyalty()` and `WorldTxn::clear_loyalty()` clearly document canonical event-sourced loyalty mutation semantics.
3. A focused regression test verifies loyalty clear => `RelationDelta::Removed`.
4. A focused regression test verifies loyalty strength update => `Removed(old)` + `Added(new)`.
5. A focused regression test verifies committed event records preserve the expected loyalty relation deltas.
6. Existing suite: `cargo test -p worldwake-core`
7. Full suite: `cargo test --workspace`
8. `cargo clippy --workspace` clean.

### Invariants

1. No behavioral code changes — loyalty mutation logic remains identical.
2. The shared weighted-relation architecture remains intact.
3. Event-sourcing via `RelationDelta` is verified at both transaction and committed-event levels.

## Test Plan

### New/Modified Tests

1. `world_txn.rs::clear_loyalty_records_removed_relation_delta` — new focused regression test.
2. `world_txn.rs::updating_loyalty_strength_records_removed_and_added_deltas` — new focused regression test.
3. `world_txn.rs::commit_preserves_loyalty_relation_deltas_in_event_log` — new focused regression test.

### Commands

1. `cargo test -p worldwake-core loyalty -- --nocapture`
2. `cargo test -p worldwake-core world_txn -- --nocapture`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Corrected the ticket first: the code already had generic weighted-relation event-sourcing and existing add-path coverage, so the scope was tightened to the real gaps.
- Added concise boundary documentation on the low-level `World` loyalty helpers and the public `WorldTxn` loyalty wrappers.
- Added focused regression coverage for loyalty removal, loyalty strength updates, and committed event-log preservation of loyalty deltas.
- Kept the current architecture intact: no loyalty-specific mutation path, no behavior change, no compatibility aliasing.
