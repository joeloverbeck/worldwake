# S103BELCLADED-002: Skip summary re-derivation when no claims pruned

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — belief pruning (worldwake-core)
**Deps**: S101 (completed), archive/tickets/S103BELCLADED-004.md (completed)

## Problem

`prune_decayed_beliefs` still calls `refresh_entity_summary_from_claims` for every entity that has claims, regardless of whether pruning actually removed anything. After `S103BELCLADED-004`, claim-backed semantic belief updates are canonicalized, so under the live runtime contract unchanged claim vectors do not need unconditional summary re-derivation on every prune pass. That leaves avoidable O(claims) summary work per retained entity.

There is one narrow correctness caveat: if `claim_confidence_threshold == 0`, saturated zero-confidence claims can remain in the store and eventually tie on the confidence term, allowing the static tie-breakers (`acquired_tick`, `claim_id`) to change the winner without a membership change. The optimization must preserve the existing full refresh behavior for that zero-threshold edge instead of pretending the broader ticket `S103BELCLADED-005` is still needed.

## Assumption Reassessment (2026-04-15)

1. `prune_decayed_beliefs` in `crates/worldwake-core/src/belief.rs` still collects all `entity_claims` keys, prunes each claim vector, removes empty vectors, and then unconditionally calls `refresh_entity_summary_from_claims` for every affected entity — verified.
2. `archive/tickets/S103BELCLADED-004.md` restored the intended semantic transport boundary for the in-scope claim-backed fields, so the earlier mixed-path blocker for this optimization is resolved — verified.
3. The live ranking math in `derive_entity_summary` is `claim_rank = (effective_claim_confidence, acquired_tick, claim_id)`, and `effective_claim_confidence` applies the same per-tick staleness slope to every claim. For the live runtime profiles that prune at `claim_confidence_threshold = 50`, winner ordering is stable for an unchanged claim set until pruning actually removes a claim — verified by code inspection plus the current profile/default usage across `worldwake-core`, `worldwake-systems`, and `worldwake-ai`.
4. The earlier rejection of `S103BELCLADED-002` was over-broad. The existing test `derive_entity_summary_applies_staleness_before_selecting_winner` proves staleness matters at a chosen tick, but it does not prove a time-only winner flip for an unchanged claim set under the live positive-threshold pruning contract.
5. A real edge still exists when `claim_confidence_threshold == 0`: once competing claims both saturate to zero confidence and remain stored, static tie-breakers can change the winner without membership change. The exact abstraction boundary under audit is therefore `entity_claims` as stored evidence and `known_entities` as a derived cache whose invalidation is membership-driven for positive thresholds, but not for the zero-threshold saturation edge.
6. Mismatch + correction: `tickets/S103BELCLADED-005.md` should not own a broad time-aware invalidation architecture. This ticket owns the real optimization, with a narrow fallback to the old full-refresh path when the threshold is zero.

## Architecture Check

1. This is the clean FND-12 optimization: compress computation only when the live ranking contract guarantees unchanged membership implies unchanged winners. The zero-threshold edge stays on the existing full-refresh path instead of adding speculative cache infrastructure.
2. No backward-compatibility shims. The optimization remains local to `prune_decayed_beliefs`; no planner-side workaround or alternate summary path is introduced.

## Verification Layers

1. Changed claim vectors still trigger summary re-derivation -> focused `worldwake-core` unit coverage
2. Unchanged claim vectors under the live positive threshold retain the same summary winner without refresh -> focused `worldwake-core` unit coverage
3. Zero-threshold saturation edge keeps the old full-refresh behavior -> focused `worldwake-core` unit coverage
4. Planner-visible behavior remains unchanged after the optimization -> `guard_theron_water_at_thornwall_finds_harvest_plan` plus full `worldwake-ai`
5. Strongest proof surface is `worldwake-core` because the invalidation rule is local to claim pruning and summary ranking

## What to Change

### 1. Track changed entities during pruning

In `prune_decayed_beliefs`, record which entities actually lost claims during the confidence-threshold retain pass.

### 2. Refresh only changed entities for positive thresholds

After pruning empty claim vectors, call `refresh_entity_summary_from_claims` only for entities whose claim vectors changed when `claim_confidence_threshold > 0`.

### 3. Preserve full refresh for the zero-threshold saturation edge

If `claim_confidence_threshold == 0`, keep the existing full refresh behavior so zero-confidence ties cannot leave stale winners in `known_entities`.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `tickets/S103BELCLADED-002.md` (modify)
- `tickets/S103BELCLADED-005.md` (modify)
- `tickets/S103BELCLADED-003.md` (modify)
- `specs/S103-belief-claim-deduplication.md` (modify)

## Out of Scope

- Canonicalizing semantic belief transport paths (`S103BELCLADED-004`)
- Social observation deduplication (`S103BELCLADED-003`)
- Broad time-aware summary cache redesign

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: changed claim vectors still re-derive summaries correctly while unchanged claim vectors preserve correct winners without refresh under the live positive-threshold contract
2. New focused test: zero-threshold saturated claims still receive full refresh and can update winner by tie-break when needed
3. Existing: `derive_entity_summary_applies_staleness_before_selecting_winner`
4. Existing: `test_prune_decayed_beliefs_removes_below_threshold`
5. Existing: `test_prune_decayed_beliefs_removes_orphan_claims`
6. Existing suite: `cargo test -p worldwake-core`
7. Existing suite: `cargo test -p worldwake-ai`
8. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. For positive claim-confidence thresholds, unchanged claim vectors do not change summary winners during pruning
2. For zero claim-confidence threshold, pruning preserves correctness by keeping the old full-refresh path
3. The optimization changes no planner-visible behavior under the live runtime profile contract

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — focused prune test covering changed vs unchanged claim vectors under positive threshold
2. `crates/worldwake-core/src/belief.rs` — focused prune test covering the zero-threshold saturation edge
3. `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` — existing Guard Theron golden remains the motivating cross-layer regression guard

### Commands

1. `cargo test -p worldwake-core --lib belief::tests::derive_entity_summary_applies_staleness_before_selecting_winner -- --exact`
2. `cargo test -p worldwake-core --lib belief::tests::test_prune_decayed_beliefs_`
3. `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots guard_theron_water_at_thornwall_finds_harvest_plan`
4. `cargo test -p worldwake-core`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-15.

- Restored `S103BELCLADED-002` as the real pruning optimization after reassessment showed the broader `S103BELCLADED-005` invalidation redesign was unnecessary under the live positive-threshold ranking contract.
- Updated `prune_decayed_beliefs` to re-derive summaries only for entities whose claim vectors actually changed when `claim_confidence_threshold > 0`.
- Preserved the old full-refresh behavior for the narrow `claim_confidence_threshold == 0` saturation edge, where zero-confidence ties can still change the winner without a membership change.
- Added focused `worldwake-core` proof for both the normal changed-entity optimization path and the zero-threshold fallback.
- Updated `tickets/S103BELCLADED-005.md`, `tickets/S103BELCLADED-003.md`, and `specs/S103-belief-claim-deduplication.md` so the ticket/spec chain matches the live contract again.

## Deviations

- `cargo test -p worldwake-ai` still fails on the unrelated existing blocker `golden_faction_ownership_producer_owner_delegation` in `crates/worldwake-ai/tests/golden_production.rs`, which is already owned by `tickets/S01PROOUTOWNCLA-013-faction-producer-owner-apple-chain-regression.md`.

## Verification Result

- Passed `cargo test -p worldwake-core --lib belief::tests::derive_entity_summary_applies_staleness_before_selecting_winner -- --exact`
- Passed `cargo test -p worldwake-core --lib belief::tests::test_prune_decayed_beliefs_`
- Passed `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots guard_theron_water_at_thornwall_finds_harvest_plan`
- Passed `cargo test -p worldwake-core`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p worldwake-ai` rerun reached the same unrelated existing failure in `golden_faction_ownership_producer_owner_delegation`
