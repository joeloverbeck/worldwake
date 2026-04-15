# S103BELCLADED-005: Add time-aware entity summary invalidation for belief pruning

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — belief summary caching/invalidation (worldwake-core)
**Deps**: S101 (completed), archive/tickets/S103BELCLADED-001.md (completed), archive/tickets/S103BELCLADED-002.md (rejected reassessment), archive/tickets/S103BELCLADED-004.md (completed)

## Problem

`prune_decayed_beliefs` currently pays a full `refresh_entity_summary_from_claims` cost for every entity with claims. The previously proposed shortcut in `S103BELCLADED-002` was invalid because `derive_entity_summary` is time-sensitive: an unchanged claim vector can still produce a different winner at a later tick as staleness penalties accumulate. The actual optimization target is therefore not “changed claim vectors only,” but “refresh only when stored evidence changed or when the current derived winner can lawfully change because time crossed a tracked boundary.”

Without that time-aware invalidation contract, any local skip risks planner-visible behavior regressions.

## Assumption Reassessment (2026-04-14)

1. `derive_entity_summary` in `crates/worldwake-core/src/belief.rs` ranks per-aspect winners by `effective_claim_confidence`, and that score depends on `current_tick` — verified.
2. The existing focused test `derive_entity_summary_applies_staleness_before_selecting_winner` proves a winner can flip between two unchanged claims solely because time advanced — verified.
3. `archive/tickets/S103BELCLADED-004.md` restored the intended semantic transport boundary for in-scope claim-backed fields, so the remaining blocker is now time-aware invalidation rather than mixed semantic write paths.
4. The exact abstraction boundary under audit is `entity_claims` as stored evidence and `known_entities` as a derived cache whose content depends on both evidence and time.
5. Any lawful optimization in this area must preserve planner-visible reads from `get_entity(...)` across arbitrary ticks, not just across prune passes where claim membership changed.
6. A changed-entity-only retain-pass optimization was attempted and rejected in `S103BELCLADED-002`; that rejected slice is a direct dependency for scoping this replacement work.
7. The likely clean design space is one of:
   - track a per-entity next-summary-transition tick and recompute only when pruning changed claims or the horizon is reached
   - or make time-sensitive summary fields derive on read instead of being stored as a long-lived cache
8. This ticket owns choosing and implementing one of those lawful designs inside `worldwake-core`. It should not reintroduce duplicate semantic storage paths or planner-layer heuristics.

## Architecture Check

1. A time-aware invalidation contract is cleaner than partial skip heuristics because it models the real cause of summary change: evidence mutation or time crossing a winner-change boundary.
2. No backward-compatibility shims. The final design should replace the unconditional refresh cost with one explicit lawful invalidation mechanism, not add fallback refresh aliases or planner-side workarounds.

## Verification Layers

1. Summary cache remains correct when time advances without claim membership changes -> focused `worldwake-core` unit tests
2. Pruning still removes below-threshold claims and orphan claim sets correctly -> existing focused prune tests
3. Planner-visible behavior remains unchanged under the new invalidation contract -> motivating golden plus full `worldwake-ai`
4. Strongest proof surface is `worldwake-core` because the contract is local to claim ranking and summary cache invalidation; golden coverage remains the cross-layer guardrail

## What to Change

### 1. Choose and encode a lawful time-aware invalidation contract

Implement one explicit mechanism in `AgentBeliefStore` for claim-backed entity summaries:

- either store enough metadata to know the next tick at which any aspect winner could change
- or stop storing time-sensitive summary content in a stale cache and derive it on read from claims

The design must make summary correctness obvious under advancing `current_tick`, not just after pruning.

### 2. Apply the invalidation contract in pruning and summary reads

Update `prune_decayed_beliefs` and any relevant summary-read path so that:

- claim removals still force re-derivation
- pure passage of time is handled by the chosen invalidation contract
- planner-visible reads do not observe stale winners

### 3. Add focused proof for time-only winner transitions

Add tests where:

- claim vectors remain unchanged
- the winning aspect flips or expires as time advances
- the store still returns the same answer as direct `derive_entity_summary(...)`

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `tickets/S103BELCLADED-005.md` (new)
- `specs/S103-belief-claim-deduplication.md` (modify if implementation narrows or clarifies the design)

## Out of Scope

- Social observation deduplication (`S103BELCLADED-003`)
- Reopening the rejected changed-entity-only optimization from `S103BELCLADED-002`
- Broad planner or candidate-generation changes

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: unchanged claim vector but advancing time still yields the correct updated summary under the store API
2. Existing: `test_prune_decayed_beliefs_removes_below_threshold`
3. Existing: `test_prune_decayed_beliefs_removes_orphan_claims`
4. Existing focused: `derive_entity_summary_applies_staleness_before_selecting_winner`
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo test -p worldwake-ai`
7. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Claim-backed summary reads remain correct as `current_tick` advances, even when claim membership is unchanged
2. The optimization compresses computation only; it must not change which claim wins for any aspect at any tick

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — focused tests for time-only summary invalidation and parity with direct `derive_entity_summary(...)`
2. `crates/worldwake-core/src/belief.rs` — existing prune tests remain the proof surface for claim-removal behavior
3. `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` — existing Guard Theron golden remains the motivating cross-layer regression guard

### Commands

1. `cargo test -p worldwake-core --lib belief::tests::derive_entity_summary_applies_staleness_before_selecting_winner -- --exact`
2. `cargo test -p worldwake-core --lib belief::tests::test_prune_decayed_beliefs_`
3. `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots guard_theron_water_at_thornwall_finds_harvest_plan`
4. `cargo test -p worldwake-core`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
