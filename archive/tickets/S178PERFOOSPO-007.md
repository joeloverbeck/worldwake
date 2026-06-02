# S178PERFOOSPO-007: SurvivalForensicExtractor spoiled-food discovery record

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — perception emits a compact lot-condition expectation-mismatch decision payload, `SurvivalForensicExtractor` derives `SpoiledFoodDiscovery` records into critical-window frames, observer summaries handle the payload, and save-format coverage includes the new payload variant.
**Deps**: `archive/tickets/S178PERFOOSPO-005.md`, `archive/tickets/S178PERFOOSPO-006.md`

## Problem

D6 records the belief-vs-observed mismatch when an agent reaches a believed-edible food lot and finds it spoiled. The forensic record feeds debug introspection (FND-29 "Why did this agent eat spoiled food?") and lets goldens prove the FND-17 expectation-violation chain. Follows S177's `SourceAcquisitionFailure` precedent — derived forensic state, never authoritative, surfaced through the existing critical-window summary.

## Assumption Reassessment (2026-06-02)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `SurvivalForensicExtractor::observe` receives the tick event log, action trace, and local survival summary, but not mutable belief snapshots. The live seam for "belief before observation vs. observation after arrival" is `record_observed_snapshot` in `crates/worldwake-systems/src/perception.rs`, where both the prior `BelievedEntityState` and the new observed snapshot are available before the belief store is updated.
2. The implementation adds `DecisionEventPayload::LotConditionExpectationMismatch` as the append-only causal carrier. Perception emits it with `EventTag::ExpectationMismatch` when an observed item lot's condition differs from the observer's prior lot-condition belief. The forensic extractor consumes that payload; it does not read authoritative perishable state.
3. `source_acquisition_failures` live on `CriticalWindowFrame`, not `LocalSurvivalStateSummary`. `SpoiledFoodDiscovery` was added to the same critical-window frame surface, with `#[serde(default)]` for older frame JSON fixtures. The S178 spec D6 wording was updated to match this live surface.
4. The extractor records only Hunger critical-window discoveries where the prior believed condition was at or above the commodity's spoiled threshold and the observed condition is below that threshold. Outcomes follow the existing source-acquisition pattern: the discovery initially records `GaveUp`, then later action traces in the same active window can update it to `AteAnyway` or `TraveledToFallback`.
5. Coverage gap classification: this ticket adds focused/unit producer coverage for perception, focused/unit consumer coverage for survival forensics, core payload roundtrip coverage, and save-format roundtrip coverage. Full-chain golden proof remains deferred to ticket 008.

## Architecture Check

1. Derived forensic state mirrors S177's `SourceAcquisitionFailure` precedent — record lives in the critical-window frame, never feeds back into authoritative state (FND-27 caches-never-truth). FND-29A append-only causal history flows through the event log; the extractor synthesizes the higher-level forensic record from a decision payload plus action traces.
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

### 2. Add to `CriticalWindowFrame`

Alongside `source_acquisition_failures`:

```rust
#[serde(default)]
pub spoiled_food_discoveries: Vec<SpoiledFoodDiscovery>,
```

### 3. Producer and extractor detection logic

In `record_observed_snapshot`, emit `DecisionEventPayload::LotConditionExpectationMismatch` when the observer's prior lot condition belief differs from the new observed condition. In `SurvivalForensicExtractor::observe`, record a `SpoiledFoodDiscovery { tick, lot, believed_condition: prior_belief, observed_condition: new_observation, outcome }` when the payload shows the agent believed the lot edible and observed it below the spoiled threshold. Determine `outcome` from action traces:
- Eat action committed on the spoiled lot → `AteAnyway`.
- Travel action started toward a different food lot → `TraveledToFallback`.
- No relevant eat or travel action appears while the critical Hunger window remains active → `GaveUp`.

The extractor reads the event log and action traces, matching the `SourceAcquisitionFailure` derived-forensic precedent.

## Files to Touch

- `crates/worldwake-core/src/decision_event_payload.rs` (modify — `LotConditionExpectationMismatchPayload` + enum variant + payload roundtrip sample)
- `crates/worldwake-core/src/lib.rs` (modify — export payload type)
- `crates/worldwake-systems/src/perception.rs` (modify — emit lot-condition expectation mismatch payload and producer test)
- `crates/worldwake-ai/src/survival_forensics.rs` (modify — `SpoiledFoodDiscovery` + `SpoiledFoodOutcome` + `CriticalWindowFrame.spoiled_food_discoveries` + extractor logic)
- `crates/worldwake-ai/src/lib.rs` (modify — export forensic types)
- `crates/worldwake-ai/src/bin/soak_seed_perf.rs` (modify — payload tag handling)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — payload owner/name/summary handling)
- `crates/worldwake-sim/src/save_load.rs` (modify — save-format version 120 and payload roundtrip coverage)
- `specs/S178-perishable-food-spoilage.md` (modify — D6 forensic surface wording)

## Out of Scope

- Goldens that exercise the extractor end-to-end (ticket 008).
- Full observer rendering of `SpoiledFoodDiscovery` records (ticket 008 owns the end-to-end golden-facing surface). This ticket only adds observer handling for the new decision payload owner/name/summary.
- Modifications to `SurvivalForensicExtractor`'s public constructor or per-agent instantiation pattern (no breaking changes).
- Disease/sickness consequence of eating spoiled food (deferred per spec Non-Goals).

## Acceptance Criteria

### Tests That Must Pass

1. `spoiled_food_discovery_recorded_when_belief_fresh_and_observed_spoiled` — extractor synthesizes record from the lot-condition expectation-mismatch event-log fixture.
2. `spoiled_food_discovery_outcome_ate_anyway_when_agent_eats_spoiled_lot` — agent eats the spoiled lot on the next decision tick; `outcome == AteAnyway`.
3. `spoiled_food_discovery_outcome_traveled_to_fallback_when_agent_seeks_other_food` — agent travels to a different food lot; `outcome == TraveledToFallback`.
4. `spoiled_food_discovery_outcome_gave_up_when_agent_idles_past_window` — agent idles past detection window; `outcome == GaveUp`.
5. `spoiled_food_discovery_does_not_fire_without_prior_belief` — agent observes a Spoiled lot with no prior belief about its freshness; no record emitted.
6. Existing: `cargo test -p worldwake-ai survival_forensics`.

### Invariants

1. `SpoiledFoodDiscovery` is derived forensic state, never authoritative (AGENTS.md FND-29A invariant; FND-27 caches-never-truth).
2. `SpoiledFoodOutcome` enum is exhaustive — every lawful response branch is named; no `Unknown` catch-all.
3. Record only fires when belief-vs-observed mismatch holds — never on direct observation of an authoritative-Spoiled lot without prior expectation (FND-17 violated-expectation precondition).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/survival_forensics.rs` `#[cfg(test)]` — 5 new unit tests (one per outcome branch + the no-prior-belief negative case).

### Commands

1. `cargo test -p worldwake-systems perception_emits_lot_condition_expectation_mismatch_when_known_lot_condition_changes`
2. `cargo test -p worldwake-ai survival_forensics::tests::spoiled_food`
3. `cargo test -p worldwake-sim save_format_version_is_120_after_lot_condition_mismatch_payload`
4. `cargo test -p worldwake-sim save_to_bytes_roundtrip_preserves_full_nondefault_state`
5. `cargo test -p worldwake-ai survival_forensics`
6. `cargo test -p worldwake-core decision_event_payload`
7. `cargo check --workspace`

## Outcome

Completed on 2026-06-02. Perception now writes a first-class lot-condition expectation-mismatch decision payload when an observer's prior lot-condition belief is contradicted by a new observation. Survival forensics derives `SpoiledFoodDiscovery` records into critical-window frames from that append-only payload, records all three lawful outcomes, and updates pending `GaveUp` records when a later eat or travel action appears in the same critical window. Observer summaries, payload roundtrip coverage, and save-format fixtures were updated for the new shared payload. Full-chain golden proof remains queued in ticket 008.
