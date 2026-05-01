# S130SURRECFRO-007: Perception-time hypothesis evaluation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — perception-system extension that writes `SurveyMemory` entries and emits `EventTag::SurveyRecorded`
**Deps**: `archive/tickets/S130SURRECFRO-001.md`, 002, 003, 004, spec `specs/S130-survey-records-frontier-disconfirmation.md` D6, D10 (automatic)

## Problem

When an agent arrives at a place under an active `ExploreLocation` goal, the perception system must evaluate the goal's hypothesis against the agent's freshly-perceived entities at the place. If the hypothesis is satisfied (e.g., agent was looking for `Apple` and a `ResourceSource` with `commodity: Apple, available_quantity > 0` is present), record `found = true`; otherwise `found = false`. The record persists in the agent's `SurveyMemory` and the event log carries an `EventTag::SurveyRecorded` event. This closes the "I checked Hillside Shelter for food and confirmed it is empty" loop that motivates the entire spec.

## Assumption Reassessment (2026-05-02)

1. `perception_system` lives at `crates/worldwake-systems/src/perception.rs:35` and runs at dispatch position 8 (`crates/worldwake-systems/src/lib.rs:104`); the AI agent tick runs after perception in the same simulation step. `is_satisfied(GoalKind::ExploreLocation { target_place, .. })` at `crates/worldwake-ai/src/goal_model.rs:1448` returns `effective_place(actor) == Some(*target_place)` and is consulted only during the AI agent tick — so the agent's `IntentionFrame.goal` is still the active `ExploreLocation` when perception evaluates the hypothesis. Dispatch order alone establishes the read window.
2. The arrival-perception path is `observe_passive_local_entities` at `crates/worldwake-systems/src/perception.rs:50-58`; it calls `collect_direct_local_observation_batch` and `apply_direct_local_observation_batch`. Survey evaluation hooks in after `apply_direct_local_observation_batch` so the agent's belief store has been updated with arrival observations before the hypothesis is evaluated.
3. Hypothesis evaluators read physical properties of co-located entities (FND-14A — Same-Tick Local Observation Is Belief-Equivalent): `ResourceSource.commodity` / `available_quantity` (`crates/worldwake-core/src/production.rs:75-83`), `ItemLot.commodity` (`crates/worldwake-core/src/items.rs:317-321`), `PlaceTag::Latrine` (`topology.rs:11-26`), `WorkstationTag::WashBasin` (`production.rs:10-20`), `SleepQualityProfile.recovery_modifier` (`sleep_episode.rs:47-62`). All are belief-equivalent reads; no FND-14 violation.
4. `Quantity` literal zero is `Quantity(0)` (codebase convention at `goal_model.rs:1498`), not `Quantity::zero()` — there is no `zero()` constructor.
5. `PerceptionProfile.observation_fidelity` at `crates/worldwake-core/src/belief.rs:2556` is the confidence source for `SurveyRecord.confidence`. `CognitiveProfile.survey_memory_capacity` (added in ticket 001) is the capacity argument for `SurveyMemory::record`.
6. D10 acknowledged as automatic: when `MayContainCommodity { commodity }` evaluates to `found = false` and the agent has `FrameAssumption::CommodityAvailableAt { commodity, place }` on the active `IntentionFrame`, the existing `record_assumption_failure` path at `crates/worldwake-ai/src/agent_tick/frame.rs:596` already fires alongside (during the next AI tick) — no new code in this spec is needed for the assumption-failure side. The two systems are orthogonal and both fire on the same arrival tick (perception writes the survey; the next AI tick records the `ExpectationMismatch`).
7. **Information-path analysis** (Assumption Reassessment item — information-path refactor): survey records have a single transport path — perception writes to `agent.SurveyMemory` directly via `txn.set_component_survey_memory`. The AI ranking layer reads via `GoalBeliefView::survey_memory()` (ticket 004's accessor). No duplicate path; no migration concern.
8. Existing perception tests at `perception.rs:1416-3215` (28+ tests) exercise `observe_passive_local_entities` and witness-event paths but do not currently set up an active `ExploreLocation` goal on the test agent. New tests in this ticket will set up the goal before triggering perception.

## Architecture Check

1. Survey writes belong in perception because they are belief-state writes triggered by perceptual events (FND-14A — same-tick co-located observation is belief-equivalent). Doing the writes in the AI tick instead would require a second perception-equivalent read, doubling the work and creating a lifecycle-ordering hazard.
2. `MayContainCommodity` evaluator checks both `ResourceSource` and `ItemLot` because both are valid commodity-bearing entities at a place — the agent looking for Apples is satisfied by either a wild apple tree (resource source) or a sack of Apples in a container (item lot).
3. `MayContainSleepSite` requires `recovery_modifier > Permille::new(1000)` (strictly better than universal default) — surveying a place with a default-quality sleep profile is not informative because every place has the universal default; only above-default recovery counts as a found sleep site (per S128 — hard dependency, satisfied).
4. `Proactive` always evaluates to `found = true` — the act of arriving satisfies proactive intent (the agent was exploring without specific expectation; arrival is success). This produces a positive survey that does not damp future re-exploration.
5. No backward-compat shim — net-new perception path; no prior survey writes existed.

## Verification Layers

1. Negative-survey arrival writes a `SurveyMemory` entry with `found = false` and emits `EventTag::SurveyRecorded` → focused unit/runtime test (perception harness with no commodity present at the target place).
2. Positive-survey arrival writes a `SurveyMemory` entry with `found = true` and emits `EventTag::SurveyRecorded` → focused unit/runtime test (perception harness with matching `ResourceSource` present).
3. `MayContainLatrine` evaluator detects the `PlaceTag::Latrine` tag → focused unit test.
4. `MayContainWashBasin` evaluator detects a workstation tagged `WashBasin` at the place → focused unit test.
5. `MayContainSleepSite` evaluator returns `true` only when `recovery_modifier > Permille::new(1000)` → focused unit tests at the boundary (`= 1000` → false, `> 1000` → true).
6. `Proactive` arrival always records `found = true` → focused unit test.
7. Arrival without an active `ExploreLocation` goal writes no survey → focused unit test (negative case).
8. `EventTag::SurveyRecorded` event-log entry carries `SurveyRecordedPayload` with full provenance (surveyor, place, hypothesis, found, confidence) → event-log delta assertion test.
9. Mixed-layer ticket — perception read (FND-14A authoritative-state read of co-located entities), `SurveyMemory` write (per-agent component mutation), `EventTag::SurveyRecorded` event-log emission. Each invariant maps to its proof surface above.

## What to Change

### 1. Hypothesis evaluator helpers

Add to `crates/worldwake-systems/src/perception.rs` (or a sibling helper module if file size warrants):

```rust
fn evaluate_hypothesis(
    world: &World,
    place: EntityId,
    hypothesis: HypothesisKind,
) -> bool {
    match hypothesis {
        HypothesisKind::MayContainCommodity { commodity } => {
            world
                .resource_sources_at(place)
                .any(|rs| rs.commodity == commodity && rs.available_quantity > Quantity(0))
                || world
                    .item_lots_at(place)
                    .any(|lot| lot.commodity == commodity)
        }
        HypothesisKind::MayContainLatrine => {
            world
                .get_component_place_tags(place)
                .map_or(false, |tags| tags.contains(&PlaceTag::Latrine))
        }
        HypothesisKind::MayContainWashBasin => {
            world
                .workstations_at(place)
                .any(|ws| ws.tag == WorkstationTag::WashBasin)
        }
        HypothesisKind::MayContainSleepSite => {
            world
                .get_component_sleep_quality_profile(place)
                .map_or(false, |p| p.recovery_modifier > Permille::new_unchecked(1000))
        }
        HypothesisKind::Proactive => true,
    }
}
```

(Helper accessor names — `resource_sources_at`, `item_lots_at`, `workstations_at`, `get_component_place_tags`, `get_component_sleep_quality_profile` — bind to whatever the current world API actually exposes for these reads. Confirm in reassessment; substitute the exact accessor or compose from existing iterators if no direct accessor exists.)

### 2. Survey-write hook in perception

In `perception_system`, after `apply_direct_local_observation_batch` and before the witness/event loop, iterate agents whose perception store updated this tick. For each:

- Read `world.get_component_intention_frame(agent)` and check whether the active `IntentionFrame.goal` is `GoalKind::ExploreLocation { target_place, hypothesis, .. }` with `target_place == world.effective_place(agent)?`.
- If yes, evaluate `evaluate_hypothesis(world, target_place, hypothesis)`.
- Read `confidence = world.get_component_perception_profile(agent)?.observation_fidelity`.
- Read `capacity = world.get_component_cognitive_profile(agent)?.survey_memory_capacity`.
- Construct `SurveyRecord { place: target_place, hypothesis, found, confidence, recorded_tick: tick }`.
- `txn.get_component_survey_memory_mut(agent)?.record(record, capacity)`.
- `txn.add_tag(EventTag::SurveyRecorded).set_payload(SurveyRecordedPayload { surveyor: agent, place: target_place, hypothesis, found, confidence })`.

D10: no extra code is needed for the `CommodityAvailableAt` assumption-failure side — the existing `record_assumption_failure` path at `crates/worldwake-ai/src/agent_tick/frame.rs:596` fires automatically during the next AI tick when the assumption is detected as violated.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify — hypothesis evaluators + survey-write hook + new tests)
- Likely: `crates/worldwake-systems/src/perception/<helper>.rs` (new — if file size warrants extracting evaluators; discover in reassessment)

## Out of Scope

- Per-tick decay of `SurveyMemory` (ticket 008)
- Ranking damping consumption of survey records (ticket 006 — already complete by this point)
- Golden coverage of the end-to-end behavioral chain (ticket 009)
- D10 explicit code — automatic via existing S122 path (`record_assumption_failure` at `agent_tick/frame.rs:596`); no new code needed
- Cross-agent survey propagation — explicitly non-goal per spec; surveys are per-agent in this spec

## Acceptance Criteria

### Tests That Must Pass

1. New: `arrival_with_negative_commodity_hypothesis_writes_negative_survey` — agent arrives at place with no Apples; `SurveyMemory` gets `found = false`; event log carries `SurveyRecorded`.
2. New: `arrival_with_positive_commodity_hypothesis_writes_positive_survey` — agent arrives at place with matching `ResourceSource`; `found = true`.
3. New: `arrival_with_item_lot_satisfies_commodity_hypothesis` — `ItemLot` of the matching commodity is sufficient.
4. New: `arrival_with_latrine_hypothesis_uses_place_tag` — `PlaceTag::Latrine` → `found = true`.
5. New: `arrival_with_wash_basin_hypothesis_uses_workstation_tag` — workstation `WashBasin` → `found = true`.
6. New: `arrival_with_sleep_site_hypothesis_requires_recovery_modifier_above_universal_default` — boundary tests at `recovery_modifier = 1000` (false) and `> 1000` (true).
7. New: `arrival_with_proactive_hypothesis_always_writes_positive_survey`.
8. New: `arrival_without_active_explore_location_writes_no_survey` — negative case.
9. New: `survey_recorded_event_payload_carries_full_provenance` — event-log delta assertion.
10. Existing: `cargo test -p worldwake-systems perception` — passes; no existing perception test exercises an active `ExploreLocation` goal, so no fixture conflicts.
11. Existing suite: `cargo test --workspace`.

### Invariants

1. Every arrival under an active `ExploreLocation` goal targeting the current effective place produces exactly one `SurveyMemory` write and one `EventTag::SurveyRecorded` event-log entry — no duplicate writes per tick.
2. Hypothesis evaluators read only physical properties of co-located entities (FND-14A) — no social, relational, or institutional facts.
3. `MayContainSleepSite` evaluator uses `recovery_modifier > Permille::new(1000)` (strictly above universal default) — agents do not record positive sleep-site surveys for default-quality places.
4. D10 (automatic): when survey is `MayContainCommodity` negative and `FrameAssumption::CommodityAvailableAt` is on the active `IntentionFrame`, the existing AI-tick `record_assumption_failure` path fires on the next tick — verified by event-log presence of both `SurveyRecorded` (this tick) and `ExpectationMismatch` (next tick) in a focused integration test.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` (`#[cfg(test)]` block) — 9 new focused/runtime tests covering each hypothesis variant + negative-control + event-payload provenance (per Acceptance Criteria 1-9).

### Commands

1. `cargo test -p worldwake-systems perception::tests::arrival`
2. `cargo test -p worldwake-systems perception`
3. `cargo test -p worldwake-systems`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
