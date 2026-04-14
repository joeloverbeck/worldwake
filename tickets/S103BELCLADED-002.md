# S103BELCLADED-002: Skip summary re-derivation when no claims pruned

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — belief pruning (worldwake-core)
**Deps**: S101 (completed)

## Problem

`prune_decayed_beliefs` calls `refresh_entity_summary_from_claims` for every entity that has claims, regardless of whether any claims were actually removed during the retain pass. This means entities whose claim sets didn't change still incur O(claims) iteration in `derive_entity_summary`. With many known entities and few actual prune events per tick, most of this work is wasted.

## Assumption Reassessment (2026-04-14)

1. `prune_decayed_beliefs` at `belief.rs:185` collects `affected_entities` from all `entity_claims` keys before pruning, then calls `refresh_entity_summary_from_claims` for every entity in that list after pruning — verified. The re-derivation loop is at lines 209-214.
2. `refresh_entity_summary_from_claims` at `belief.rs:118` calls `derive_entity_summary` which iterates all claims for the entity — verified.
3. `effective_claim_confidence` at `belief.rs:1983` is a private function used inside the retain predicate — verified, accessible within the same module.
4. Existing tests: `test_prune_decayed_beliefs_removes_below_threshold` (line 4117), `test_prune_decayed_beliefs_removes_orphan_claims` (line 4211), `test_refresh_entity_summary_from_claims_preserves_presentation_history` (line 4284) — all in `belief.rs` test module.

## Architecture Check

1. Tracking which entities actually had claims removed is a pure computation optimization (FND-12). The change does not alter what gets pruned or what the derived summary contains — it only skips re-derivation when the input hasn't changed. This is the minimal change: no new data structures, just a `len_before != len_after` check per entity.
2. No backward-compatibility shims. The function signature is unchanged.

## Verification Layers

1. Entities with pruned claims get re-derived summaries → existing `test_prune_decayed_beliefs_removes_below_threshold` continues to pass
2. Entities with no pruned claims skip re-derivation → new focused test asserting summary call count or equivalently that the summary is unchanged
3. Golden tests pass unchanged → `cargo test -p worldwake-ai`

## What to Change

### 1. Track changed entities during the retain pass

In `prune_decayed_beliefs`, replace the unconditional re-derivation loop with one that only processes entities whose claim count decreased:

```rust
let mut changed_entities = Vec::new();
for entity in &affected_entities {
    let Some(claims) = self.entity_claims.get_mut(entity) else {
        continue;
    };
    let len_before = claims.len();
    claims.retain(|claim| {
        effective_claim_confidence(claim, current_tick, &profile.confidence_policy)
            >= claim_confidence_threshold
    });
    if claims.len() < len_before {
        changed_entities.push(*entity);
    }
}
self.entity_claims.retain(|_, claims| !claims.is_empty());
for entity in changed_entities {
    self.refresh_entity_summary_from_claims(
        entity,
        current_tick,
        &profile.confidence_policy,
    );
}
```

This replaces the current pattern where claims are pruned in one loop and then every entity gets re-derived in a separate loop.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)

## Out of Scope

- Changing what gets pruned (confidence thresholds, decay rates)
- Claim deduplication (S103BELCLADED-001)
- Social observation deduplication (S103BELCLADED-003)
- Changing `derive_entity_summary` logic

## Acceptance Criteria

### Tests That Must Pass

1. New: entity with no claims pruned retains its existing summary unchanged (no re-derivation side effects)
2. New: entity with claims pruned gets an updated summary reflecting the remaining claims
3. Existing: `test_prune_decayed_beliefs_removes_below_threshold` passes unchanged
4. Existing: `test_prune_decayed_beliefs_removes_orphan_claims` passes unchanged
5. Existing suite: `cargo test -p worldwake-core` and `cargo test -p worldwake-ai`

### Invariants

1. `prune_decayed_beliefs` produces identical world-visible results (same claims removed, same summaries derived) — only the computation path changes
2. Entities with all claims pruned are still removed from `entity_claims` via the `.retain()` call

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` (test module) — `prune_decayed_beliefs_skips_re_derivation_when_no_claims_removed`: create an entity with fresh claims (confidence above threshold), call `prune_decayed_beliefs`, assert the summary is unchanged and no unnecessary work occurred
2. `crates/worldwake-core/src/belief.rs` (test module) — `prune_decayed_beliefs_re_derives_only_changed_entities`: create two entities — one with stale claims and one with fresh claims — call `prune_decayed_beliefs`, assert the stale entity's summary updates while the fresh entity's summary remains identical

### Commands

1. `cargo test -p worldwake-core prune_decayed_beliefs`
2. `cargo test -p worldwake-core`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
