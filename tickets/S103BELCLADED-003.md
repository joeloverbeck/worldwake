# S103BELCLADED-003: Social observation deduplication in `record_social_observation`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — social observation storage (worldwake-core)
**Deps**: S101 (completed)

## Problem

Social observations accumulate unbounded — 5,228 by tick 500 in the T30 soak world. `record_social_observation` unconditionally appends every new observation. When an agent repeatedly witnesses the same social event (e.g., agent A cooperating with agent B at the same location), each sighting creates a new record even though it conveys no new information. The older observation is strictly superseded by the newer one.

## Assumption Reassessment (2026-04-14)

1. `record_social_observation` at `belief.rs:146` unconditionally pushes: `self.social_observations.push(observation)` — verified.
2. `SocialObservation` (`belief.rs:2064`) has fields `detail: SocialObservationDetail`, `place: EntityId`, `observed_tick: Tick`, `source: PerceptionSource` — verified. There is no `subject` field; observed entities are embedded in `SocialObservationDetail` variants.
3. `SocialObservationDetail` (`belief.rs:2079`) derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize` — verified. The full `detail` value is usable as a dedup key, naturally encoding both the kind of social event and the specific entities involved.
4. `SocialObservationDetail::kind()` method (`belief.rs:2111`) returns `SocialObservationKind` discriminant — verified. The dedup key uses the full `detail` value (not just `kind()`), which distinguishes `WitnessedCooperation { actor: A, counterpart: B }` from `WitnessedCooperation { actor: C, counterpart: D }`.
5. Existing test: `record_social_observation_appends_to_list` (line 3283) — tests the current unconditional append behavior. This test will need modification to reflect dedup semantics.
6. `social_observations` is stored as `Vec<SocialObservation>` on `AgentBeliefStore` (line 51) — verified.

## Architecture Check

1. Deduplication by full `detail` value is the minimal change. It preserves distinct observations about different entity pairs (e.g., cooperation between A-B vs C-D are separate records) while replacing repeated sightings of the same event with the newest observation. The bound on social observation count becomes `|distinct_social_events_witnessed|` — a physical dampener (FND-11) determined by the diversity of social interactions the agent encounters.
2. No backward-compatibility shims. The `record_social_observation` signature is unchanged; callers are unaffected.

## Verification Layers

1. Same-detail observations are replaced → focused unit test asserting observation count stays 1 after repeated same-detail observations
2. Different-detail observations coexist → focused unit test asserting both observations present
3. The newer observation's `observed_tick` is preserved → focused unit test asserting the surviving observation has the later tick
4. Golden tests pass unchanged → `cargo test -p worldwake-ai`

## What to Change

### 1. Modify `record_social_observation` to replace instead of append

When a new social observation arrives with the same `detail` as an existing one, replace the older observation (keep the one with the higher `observed_tick`):

```rust
pub fn record_social_observation(&mut self, observation: SocialObservation) {
    if let Some(existing) = self
        .social_observations
        .iter_mut()
        .find(|o| o.detail == observation.detail)
    {
        if observation.observed_tick >= existing.observed_tick {
            *existing = observation;
        }
    } else {
        self.social_observations.push(observation);
    }
}
```

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)

## Out of Scope

- Changing activation-based social observation pruning in `prune_decayed_beliefs`
- Entity claim deduplication (S103BELCLADED-001)
- Amortized pruning optimization (S103BELCLADED-002)
- Changing `SocialObservation` or `SocialObservationDetail` struct definitions

## Acceptance Criteria

### Tests That Must Pass

1. New: same-detail observation replaces existing (observation count stays 1, newer tick preserved)
2. New: different-detail observations coexist (observation count is 2)
3. New: older observation does not replace newer one (keeps higher `observed_tick`)
4. Modified: `record_social_observation_appends_to_list` updated to reflect dedup behavior for same-detail case
5. Existing suite: `cargo test -p worldwake-core` and `cargo test -p worldwake-ai`

### Invariants

1. Distinct social events (different `detail` values) are always preserved as separate observations
2. Repeated sightings of the same social event keep only the most recent observation
3. Deduplication does not change what `prune_decayed_beliefs` removes — it only reduces the input set

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` (test module) — `record_social_observation_replaces_same_detail`: record two observations with identical `detail` but different `observed_tick`, assert only one remains with the newer tick
2. `crates/worldwake-core/src/belief.rs` (test module) — `record_social_observation_preserves_different_details`: record a `WitnessedCooperation { A, B }` and a `CoPresence { C }`, assert both remain
3. `crates/worldwake-core/src/belief.rs` (test module) — `record_social_observation_keeps_newer_over_older`: record a newer observation first, then an older one with the same detail, assert the newer one is preserved
4. `crates/worldwake-core/src/belief.rs` (test module) — modify `record_social_observation_appends_to_list` to use distinct `detail` values so it tests the non-dedup path

### Commands

1. `cargo test -p worldwake-core record_social_observation`
2. `cargo test -p worldwake-core`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
