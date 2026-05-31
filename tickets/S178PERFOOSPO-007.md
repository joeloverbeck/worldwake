# S178PERFOOSPO-007: SurvivalForensicExtractor spoiled-food discovery record

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `SurvivalForensicExtractor` gains `SpoiledFoodDiscovery` record type and extraction logic. `LocalSurvivalStateSummary` gains `spoiled_food_discoveries` field.
**Deps**: 005

## Problem

D6 records the belief-vs-observed mismatch when an agent reaches a believed-edible food lot and finds it spoiled. The forensic record feeds debug introspection (FND-29 "Why did this agent eat spoiled food?") and lets goldens prove the FND-17 expectation-violation chain. Follows S177's `SourceAcquisitionFailure` precedent — derived forensic state, never authoritative, surfaced through the existing critical-window summary.

## Assumption Reassessment (2026-05-31)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `SurvivalForensicExtractor` at `crates/worldwake-ai/src/survival_forensics.rs:278`. `SourceAcquisitionFailure` precedent at lines 105-123: struct with `tick`, `source`, `cause: SourceFailureCause { Depleted | QualityRejected }`, `outcome: SourceFailureOutcome { DrankAnyway | TraveledToFallback | GaveUp }`. Added to `LocalSurvivalStateSummary.source_acquisition_failures: Vec<SourceAcquisitionFailure>` at line 55. Extractor instantiated per agent at `crates/worldwake-cli/src/bin/observer.rs:5829` and in `crates/worldwake-ai/tests/scenarios/survival_baseline.rs:125`. Reads event log + belief snapshots during tick. `#[cfg(test)]` boundary at line 952.
2. Spec D6 verified against current `specs/S178-perishable-food-spoilage.md`. FND-29A — append-only event-log history flows through `EventTag::ItemSpoiled` (ticket 003); the extractor synthesizes the higher-level forensic event from raw lineage + belief deltas, never mutating authoritative state. The forensic record is derived, never authoritative.
3. Shared abstraction boundary: the `SurvivalForensicExtractor::observe` tick-level event-log reader. The extractor must detect (a) the agent observed a lot whose belief-stored `last_observed_condition` was at or above `stale_threshold` (the agent believed it edible), (b) the lot's authoritative `condition` at the observation tick is below `spoiled_threshold` (it's actually Spoiled), (c) the outcome of the agent's subsequent action (eat-anyway / travel-to-fallback / give-up).
4. Coverage gap classification (precision-rules §3): missing focused/unit coverage for the extractor write (this ticket adds it); missing golden coverage in ticket 008's `survival-food-spoilage-cache.ron`. Both layers are required — focused unit tests prove the extractor's detection logic against synthetic event-log fixtures; the golden proves the full chain end-to-end.

## Architecture Check

1. Derived forensic state mirrors S177's `SourceAcquisitionFailure` precedent — record lives in the critical window summary, never feeds back into authoritative state (FND-27 caches-never-truth). FND-29A append-only causal history flows through the event log; the extractor synthesizes the higher-level event from raw lineage + belief deltas.
2. Outcome enum captures the three lawful response branches (`AteAnyway`, `TraveledToFallback`, `GaveUp`) — enables goldens (ticket 008) to assert the desperation gate fired correctly via the outcome value rather than re-inspecting low-level action traces.

## Verification Layers

1. `SpoiledFoodDiscovery` written when agent observes a believed-fresh-but-actually-spoiled lot → focused unit test on the extractor against a synthetic event-log fixture.
2. Each outcome branch (`AteAnyway` / `TraveledToFallback` / `GaveUp`) is detected correctly from the agent's subsequent action → 3 focused unit tests, one per outcome.
3. Belief-vs-observed mismatch precondition is required for record emission → focused unit test asserting no record fires when the agent had no prior belief about the lot's freshness (the agent just discovered it Spoiled with no expectation).
4. Full-chain integration proof deferred to ticket 008's cache golden.

## What to Change

### 1. New record type and outcome enum

In `crates/worldwake-ai/src/survival_forensics.rs`, add alongside `SourceAcquisitionFailure` (lines 105-123):

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpoiledFoodDiscovery {
    pub tick: Tick,
    pub lot: EntityId,
    pub believed_condition: Permille,
    pub observed_condition: Permille,
    pub outcome: SpoiledFoodOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpoiledFoodOutcome {
    AteAnyway,
    TraveledToFallback,
    GaveUp,
}
```

### 2. Add to `LocalSurvivalStateSummary`

At line 55 of `survival_forensics.rs`, alongside `source_acquisition_failures`:

```rust
#[serde(default)]
pub spoiled_food_discoveries: Vec<SpoiledFoodDiscovery>,
```

### 3. Extractor detection logic

In `SurvivalForensicExtractor::observe`, add detection. When the agent's perception updates `last_observed_condition` for a lot AND the prior belief-stored value was at or above `stale_threshold` AND the new observed value is below `spoiled_threshold`, record a `SpoiledFoodDiscovery { tick, lot, believed_condition: prior_belief, observed_condition: new_observation, outcome }`. Determine `outcome` from the agent's subsequent action by inspecting the next-tick action trace:
- Eat action committed on the spoiled lot → `AteAnyway`.
- Travel action started toward a different food lot → `TraveledToFallback`.
- No relevant action in a configurable window (e.g., 10 ticks) → `GaveUp`.

The detection logic reads the event log (perception updates) and belief snapshots — same surfaces `SourceAcquisitionFailure` already consumes per S177's precedent.

## Files to Touch

- `crates/worldwake-ai/src/survival_forensics.rs` (modify — `SpoiledFoodDiscovery` + `SpoiledFoodOutcome` + `LocalSurvivalStateSummary.spoiled_food_discoveries` + extractor logic)

## Out of Scope

- Goldens that exercise the extractor end-to-end (ticket 008).
- Observer rendering of the new record (automatic — observer already iterates `LocalSurvivalStateSummary` via existing infrastructure; the new field renders without observer-side code change once it lands).
- Modifications to `SurvivalForensicExtractor`'s public constructor or per-agent instantiation pattern (no breaking changes).
- Disease/sickness consequence of eating spoiled food (deferred per spec Non-Goals).

## Acceptance Criteria

### Tests That Must Pass

1. `spoiled_food_discovery_recorded_when_belief_fresh_and_observed_spoiled` — extractor synthesizes record from event-log + belief snapshot fixture.
2. `spoiled_food_discovery_outcome_ate_anyway_when_agent_eats_spoiled_lot` — agent eats the spoiled lot on the next decision tick; `outcome == AteAnyway`.
3. `spoiled_food_discovery_outcome_traveled_to_fallback_when_agent_seeks_other_food` — agent travels to a different food lot; `outcome == TraveledToFallback`.
4. `spoiled_food_discovery_outcome_gave_up_when_agent_idles_past_window` — agent idles past detection window; `outcome == GaveUp`.
5. `spoiled_food_discovery_does_not_fire_without_prior_belief` — agent observes a Spoiled lot with no prior belief about its freshness; no record emitted.
6. Existing: `cargo test -p worldwake-ai survival_forensics`.

### Invariants

1. `SpoiledFoodDiscovery` is derived forensic state, never authoritative (CLAUDE.md FND-29A invariant; FND-27 caches-never-truth).
2. `SpoiledFoodOutcome` enum is exhaustive — every lawful response branch is named; no `Unknown` catch-all.
3. Record only fires when belief-vs-observed mismatch holds — never on direct observation of an authoritative-Spoiled lot without prior expectation (FND-17 violated-expectation precondition).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/survival_forensics.rs` `#[cfg(test)]` — 5 new unit tests (one per outcome branch + the no-prior-belief negative case).

### Commands

1. `cargo test -p worldwake-ai survival_forensics::tests::spoiled_food`
2. `cargo test --workspace`
3. `./scripts/verify.sh`
