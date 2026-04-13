# S101ACTBASBEL-003: PerceptionProfile migration + pruning replacement

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — PerceptionProfile field replacement, pruning logic replacement, call site updates in worldwake-systems
**Deps**: S101ACTBASBEL-001, S101ACTBASBEL-002

## Problem

The hard-capacity eviction system (`enforce_capacity`, `enforce_entity_claim_capacity`, `entity_eviction_tier`, `within_retention_window`) uses numeric clamps that violate FND-11. This ticket replaces it with activation-threshold pruning using the helpers from ticket 001 and the ring buffer from ticket 002. It also migrates PerceptionProfile from capacity-based fields to activation-based fields, updates all call sites to pass `HomeostaticNeeds`, and migrates scenario RON files.

## Assumption Reassessment (2026-04-13)

1. `PerceptionProfile` at `crates/worldwake-core/src/belief.rs:2181-2191` currently has 9 fields. 4 to remove: `entity_memory_capacity`, `entity_claim_capacity`, `memory_retention_ticks`, `infrastructure_retention_ticks`. 5 to keep: `observation_fidelity`, `confidence_policy`, `institutional_memory_capacity`, `consultation_speed_factor`, `contradiction_tolerance`. Grep confirms 121 occurrences across 42 files — most are field reads, not struct literal constructions.
2. `enforce_capacity` at `belief.rs:178`, `enforce_entity_claim_capacity` at `belief.rs:227`, `entity_eviction_tier` at `belief.rs:1974`, `within_retention_window` at `belief.rs:2261` — all exist and will be removed.
3. Call sites confirmed at: `perception.rs:224`, `perception.rs:510`, `epistemic_actions.rs:355`, `tell_actions.rs:624`, `tell_actions.rs:657`. All are runtime (before `#[cfg(test)]` boundaries). One test-only call at `tell_actions.rs:2343`.
4. `effective_claim_confidence` at `belief.rs:2004` returns `u16`. Used for claim confidence threshold pruning.
5. `HomeostaticNeeds` component access: call sites use `txn` or `world` references that can read agent components. The worldwake-systems crate depends on worldwake-core where `HomeostaticNeeds` is defined.
6. Scenario files: `scenarios/cli-evaluation.ron` has `entity_memory_capacity`, `entity_claim_capacity`, `memory_retention_ticks`, `infrastructure_retention_ticks` at lines 92-95 and 332-335. `scenarios/default.ron` has `demand_memory_retention_ticks` (different field, unrelated to PerceptionProfile — not in scope).
7. `compute_activation` and `salience_boost` from ticket 001 exist after that ticket completes.
8. After ticket 002, `BelievedEntityState` has `presentation_ticks` and `presentation_tick_count` fields, and `enforce_capacity` temporarily uses `last_observed_tick()`.

## Architecture Check

1. Unified pruning model: all memory (entities, social observations, claims) decays via activation threshold or confidence threshold — no more tiered eviction tiers or hard capacity counts. FND-11 compliant (physical dampeners, not numeric clamps). FND-16 compliant (gradual decay, not binary).
2. No backward-compatibility shims. Old fields removed, old functions removed, scenario files updated. FND-28 compliant.
3. `prune_decayed_beliefs` signature adds `&HomeostaticNeeds` parameter — this is FND-26 compliant (reads state from components, no cross-system calls).

## Verification Layers

1. Entity activation pruning correctness → focused unit test (`test_prune_decayed_beliefs_removes_below_threshold`)
2. Social observation activation pruning → focused unit test (`test_social_observation_activation_pruning`)
3. Claim confidence threshold pruning → focused unit test (claim below threshold removed)
4. Orphan claim cleanup → focused unit test (claims for pruned entities removed)
5. PerceptionProfile Default correctness → focused unit test (new defaults compile and have expected values)
6. Scenario deserialization → `cargo test --workspace` (RON parsing of updated scenarios)

## What to Change

### 1. Modify PerceptionProfile struct and Default

In `crates/worldwake-core/src/belief.rs`, replace the 4 old fields with 5 new fields:

```rust
pub struct PerceptionProfile {
    pub observation_fidelity: Permille,
    pub confidence_policy: BeliefConfidencePolicy,
    pub institutional_memory_capacity: u32,
    pub consultation_speed_factor: Permille,
    pub contradiction_tolerance: Permille,
    // New activation-based fields:
    pub entity_activation_threshold: Permille,
    pub claim_confidence_threshold: Permille,
    pub observation_buffer_capacity: u8,
    pub need_salience_boost: Permille,
    pub need_salience_urgency_threshold: Permille,
}
```

Update `Default` impl with values from spec: threshold=100, claim_confidence=50, buffer=5, boost=500, urgency=500.

### 2. Replace enforce_capacity with prune_decayed_beliefs

Remove `enforce_capacity`, `enforce_entity_claim_capacity`, `entity_eviction_tier`, `within_retention_window`.

Add `prune_decayed_beliefs` on `AgentBeliefStore`:

```rust
pub fn prune_decayed_beliefs(
    &mut self,
    profile: &PerceptionProfile,
    current_tick: Tick,
    agent_needs: &HomeostaticNeeds,
) {
    // 1. Prune social observations below activation threshold
    // 2. Prune claims below confidence threshold
    // 3. Prune entities below activation threshold (with salience boost)
    // 4. Remove orphaned claims for pruned entities
}
```

Uses `compute_activation` and `salience_boost` from ticket 001.

### 3. Update record_entity_snapshot_claims buffer_capacity

Now that PerceptionProfile has `observation_buffer_capacity`, update `record_entity_snapshot_claims` (or its callers) to pass the actual profile value instead of the temporary constant from ticket 002.

### 4. Update 5 call sites in worldwake-systems

Each call site changes from `store.enforce_capacity(&profile, tick)` to `store.prune_decayed_beliefs(&profile, tick, &needs)` where `needs` is read from the agent's `HomeostaticNeeds` component.

Call sites:
- `crates/worldwake-systems/src/perception.rs:224` (process_witness_event)
- `crates/worldwake-systems/src/perception.rs:510` (apply_direct_local_observation_batch)
- `crates/worldwake-systems/src/epistemic_actions.rs:355` (process_ask_witness_action)
- `crates/worldwake-systems/src/tell_actions.rs:624` (process_tell_action)
- `crates/worldwake-systems/src/tell_actions.rs:657` (process_tell_action)

Also update test-only call at `tell_actions.rs:2343`.

### 5. Update PerceptionProfile construction sites in tests

All test files that construct `PerceptionProfile { entity_memory_capacity: N, ... }` must be updated to use the new fields. Files using `PerceptionProfile::default()` need no changes.

Grep for `PerceptionProfile {` to find all construction sites. Major locations:
- `crates/worldwake-core/src/belief.rs` (test block — ~5 sites)
- `crates/worldwake-systems/src/perception.rs` (test block — ~3 sites)
- `crates/worldwake-systems/src/tell_actions.rs` (test block — ~1 site)
- `crates/worldwake-ai/tests/golden_*.rs` files (~40+ sites across ~20 files)
- `crates/worldwake-sim/src/` (~2 sites)

### 6. Update scenario RON files

In `scenarios/cli-evaluation.ron`, replace old perception profile fields with new activation-based fields at lines 92-95 and 332-335.

### 7. Unit tests for pruning

In `crates/worldwake-core/src/belief.rs` test block:
- `test_prune_decayed_beliefs_removes_below_threshold` — entity with old observation pruned
- `test_social_observation_activation_pruning` — social observation beyond activation threshold pruned
- `test_claim_confidence_threshold_prunes_stale_claims` — claim below confidence threshold removed
- `test_prune_preserves_high_activation_entities` — recently observed entities kept
- `test_prune_salience_boost_preserves_items` — item entities with high agent need survive longer

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify) — PerceptionProfile struct+Default, prune_decayed_beliefs, remove old functions, tests
- `crates/worldwake-systems/src/perception.rs` (modify) — 2 call sites + test construction sites
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify) — 1 call site
- `crates/worldwake-systems/src/tell_actions.rs` (modify) — 2 call sites + test construction site
- `crates/worldwake-cli/src/scenario/types.rs` (modify) — if PerceptionProfile serde fields need annotation
- `scenarios/cli-evaluation.ron` (modify) — old field names to new
- `crates/worldwake-ai/tests/golden_*.rs` (modify) — PerceptionProfile construction sites (~20 files)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify) — PerceptionProfile construction sites
- `crates/worldwake-systems/tests/e15_information_integration.rs` (modify) — PerceptionProfile construction sites
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify) — if profile field reads need updating
- `crates/worldwake-sim/src/institutional_knowledge_trace.rs` (modify) — if profile field reads need updating

## Out of Scope

- BelievedEntityState struct changes (ticket 002 — already done)
- compute_activation and salience_boost helper functions (ticket 001 — already done)
- Golden E2E tests for activation decay (ticket 004)
- Commodity-specific salience mapping (spec non-goal)
- Variable decay exponent (spec non-goal)
- Episodic memory or autobiographical recall (spec non-goal)
- Forgetting curve visualization or debug tooling (spec non-goal)

## Acceptance Criteria

### Tests That Must Pass

1. `test_prune_decayed_beliefs_removes_below_threshold` — stale entities pruned
2. `test_social_observation_activation_pruning` — social observations pruned by activation
3. `test_claim_confidence_threshold_prunes_stale_claims` — stale claims pruned
4. `test_prune_preserves_high_activation_entities` — recent entities kept
5. `test_prune_salience_boost_preserves_items` — item salience boost works
6. Existing suite: `cargo test --workspace` — all existing tests pass after migration
7. `cargo clippy --workspace --all-targets -- -D warnings` — no new warnings

### Invariants

1. No hard capacity number exists in PerceptionProfile (entity_memory_capacity, entity_claim_capacity removed)
2. No time-window retention check exists (memory_retention_ticks, infrastructure_retention_ticks removed, within_retention_window removed)
3. All memory pruning uses activation threshold — unified model for entities and social observations
4. Claim pruning uses confidence threshold — no hard count limit
5. `prune_decayed_beliefs` is called at exactly the same 5 runtime sites as the old `enforce_capacity`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — `test_prune_decayed_beliefs_removes_below_threshold`, `test_social_observation_activation_pruning`, `test_claim_confidence_threshold_prunes_stale_claims`, `test_prune_preserves_high_activation_entities`, `test_prune_salience_boost_preserves_items`
2. All existing tests with `PerceptionProfile` construction — modified to use new field names

### Commands

1. `cargo test -p worldwake-core -- test_prune`
2. `cargo test -p worldwake-core -- test_social_observation_activation`
3. `cargo test -p worldwake-core -- test_claim_confidence_threshold`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`
