# S177WATSRCQUA-007: Survival forensics `SourceAcquisitionFailure` record

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai/survival_forensics` (new `SourceAcquisitionFailure` derived record + `SourceFailureCause`/`SourceFailureOutcome` enums; extension to `SurvivalForensicExtractor.observe()`)
**Deps**: `archive/tickets/S177WATSRCQUA-004.md`

## Problem

The spec's D7 deliverable adds a derived forensic record to `SurvivalForensicExtractor` that surfaces when an agent's water-acquisition attempt failed (depleted source or quality-driven rejection). This complements the existing `DegradedSelfCareOpportunity` record (`crates/worldwake-ai/src/survival_forensics.rs:75-81`) which covers basin/latrine failures. Without this record, the critical-window forensics view cannot explain "the agent's thirst critical window includes 3 attempts at depleted/muddy sources" — debuggability (FND-29A) is incomplete for the new quality axis.

## Assumption Reassessment (2026-05-31)

1. `SurvivalForensicExtractor` at `crates/worldwake-ai/src/survival_forensics.rs:250-328` accumulates `CriticalWindowFrame`s per need during critical windows and flushes `CriticalWindowReport` on need recovery. The `observe()` method at lines 268-310 builds each frame.
2. Existing forensic record `DegradedSelfCareOpportunity` at lines 75-81: `{ tick, facility, cause: DegradedSelfCareCause, outcome: DegradedSelfCareOutcome }`. Causes are `BasinTooDirty, BasinDry, LatrineFull`; outcomes are `WildernessRelief, Cleaned, Queued, DidNothing`. The extraction fn `degraded_self_care_opportunities()` at lines 423-488 reads action-trace events at the tick.
3. The new `SourceAcquisitionFailure` record's surface is derived from: (a) `EventTag::SourceExpectationFailure` emissions (already live — see `production_actions.rs:1228` writing `record_failed_source_attempt` paired with the event emission in `agent_tick/mod.rs:1051-1052`); (b) `EventTag::ResourceSourceQualityObserved` emissions (added by ticket 004) where the observed quality matches a "rejection" condition. The extractor reads these from the action-trace / event-log snapshot.
4. Per the reassessed spec D7, the cause set is `Depleted` and `QualityRejected` — the speculative "Contested" cause was dropped because queue contention is already surfaced via `wait_factor_permille` in source_composite_rank and `observe_wait` on `ReliabilityRecord`. Adding a third cause for contention here would duplicate the existing queue substrate.
5. The outcome set per spec D7: `DrankAnyway, TraveledToFallback, GaveUp`. The outcome is reconstructed from subsequent action-trace events in the critical window — what the agent did after the failure observation.
6. Shared abstraction boundary: the `CriticalWindowFrame`'s `source_acquisition_failures: Vec<SourceAcquisitionFailure>` collection (new field). The frame already carries `local_authoritative_summary: LocalSurvivalStateSummary` and other forensic accumulators at line 52 per S120's archived spec — adding a new collection follows the same pattern.
7. Adjacent contradictions: the existing `degraded_self_care_opportunities()` extraction is event-driven (reads action-trace at the tick); the new extraction is event-driven (reads `SourceExpectationFailure` + `ResourceSourceQualityObserved` at the tick). Symmetry preserved.
8. SAVE_FORMAT_VERSION: this ticket adds derived/forensic state that may or may not be authoritative depending on whether `CriticalWindowReport` is serialized into the world snapshot. Per the spec's Stored State table, `SourceAcquisitionFailure` is classified "Derived forensic state" — not authoritative. Verify whether `CriticalWindowFrame` or `CriticalWindowReport` is in the serialized `SimulationState`:
   - If serialized: needs `#[serde(default)]` on the new field but no version bump (additive trace data).
   - If trace-only (event-log derived on demand): no serialization concern.
   - Grep `crates/worldwake-sim/src/save_load.rs` and `crates/worldwake-core/src/delta.rs` for `CriticalWindowFrame` / `CriticalWindowReport` references during implementation. If absent, no bump.

## Architecture Check

1. Derived forensic record (vs. authoritative state) follows FND-3 — the failure facts are derived from event-log + action-trace, not stored authority. FND-27: derived summaries are caches over source state.
2. Cause set `{Depleted, QualityRejected}` (vs. wider speculative set) follows YAGNI and the spec's reassessment-driven scope narrowing — adding causes for unproven scenarios is dead surface.
3. Outcome reconstruction from subsequent action-trace events (vs. recording outcome at the time of failure) — the agent's chosen response to the failure is observable from what they did next, not from the failure event itself. FND-29A inspectability: the causal chain is reconstructable.

## Verification Layers

1. New `SourceAcquisitionFailure` record fields compile and roundtrip — focused unit test in `survival_forensics.rs`.
2. Extraction fires for `SourceExpectationFailure` event: focused integration test seeds a critical-thirst window with a harvest-start-failure event, runs `observe()`, asserts the frame's `source_acquisition_failures` carries `cause: Depleted`.
3. Extraction fires for `ResourceSourceQualityObserved` with a quality the agent's tolerance rejects: focused test seeds Muddy quality observation + a tolerance that floors muddy ranking below the agent's pursuit threshold, asserts the frame carries `cause: QualityRejected`.
4. Outcome reconstruction: focused integration test runs a full critical window where the agent observes a depleted source, travels to a fallback, and successfully drinks; asserts `outcome: TraveledToFallback`.
5. Negative case: clean-water acquisition that succeeds produces no `SourceAcquisitionFailure` record — focused test asserts the collection is empty for the happy path.

## What to Change

### 1. Add `SourceAcquisitionFailure` record + enums

`crates/worldwake-ai/src/survival_forensics.rs` (near existing `DegradedSelfCareOpportunity` at lines 75-96):

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceAcquisitionFailure {
    pub tick: Tick,
    pub source: EntityId,
    pub cause: SourceFailureCause,
    pub outcome: SourceFailureOutcome,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SourceFailureCause {
    /// `available_quantity == 0` at extraction start; emits `EventTag::SourceExpectationFailure`.
    Depleted,
    /// Observed quality discounted below the agent's pursuit threshold via
    /// `WaterToleranceProfile`; emits `EventTag::ResourceSourceQualityObserved` with
    /// a quality value that, after tolerance scaling, fails to clear the ranking floor.
    QualityRejected,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SourceFailureOutcome {
    /// Agent drank the unsatisfactory water anyway (e.g., muddy with no fallback).
    DrankAnyway,
    /// Agent traveled to a believed-better fallback source.
    TraveledToFallback,
    /// No fallback met threshold; agent gave up or pursued an unrelated goal.
    GaveUp,
}
```

### 2. Extend `CriticalWindowFrame`

Add a new field `source_acquisition_failures: Vec<SourceAcquisitionFailure>` to `CriticalWindowFrame`. Use `#[serde(default)]` if the frame is serialized. Follow the pattern of `DegradedSelfCareOpportunity`-bearing fields if present.

### 3. New extraction fn `source_acquisition_failures()`

Analogous to `degraded_self_care_opportunities()` at lines 423-488. Reads action-trace + event-log at the tick window, matches on `EventTag::SourceExpectationFailure` and `EventTag::ResourceSourceQualityObserved` payloads, classifies the cause, and reconstructs the outcome from subsequent action-trace events in the critical window:

```rust
fn source_acquisition_failures(
    tick: Tick,
    action_trace_snapshot: &ActionTraceSnapshot,
    event_log: &EventLog,
    agent: EntityId,
    /* ... tolerance profile for QualityRejected classification ... */
) -> Vec<SourceAcquisitionFailure> {
    // Iterate events at the tick, filter for SourceExpectationFailure + ResourceSourceQualityObserved
    // matching agent, classify cause, reconstruct outcome by scanning subsequent action-trace events.
}
```

The function signature and tolerance-profile parameter detail should be pinned during implementation by reading the existing `degraded_self_care_opportunities` extractor closely.

### 4. Integrate into `SurvivalForensicExtractor.observe()`

`crates/worldwake-ai/src/survival_forensics.rs:268-310`: at the frame-building site, call `source_acquisition_failures(...)` and attach to the frame.

### 5. (Conditional) `SAVE_FORMAT_VERSION` bump

Determine during implementation whether `CriticalWindowFrame` is serialized in `SimulationState`:

```bash
grep -rn "CriticalWindowFrame\|CriticalWindowReport" crates/worldwake-sim/src/save_load.rs crates/worldwake-core/src/delta.rs
```

If absent from both paths, the type is decision-trace/forensic-only and no version bump is needed; the `#[serde(default)]` annotation covers trace serialization compatibility. If present, this ticket bumps 115→116 and the cascade extends. (Note: this would require updating the Merge-Order Constraints in the Step 6 summary.)

## Files to Touch

- `crates/worldwake-ai/src/survival_forensics.rs` (modify — new record + enums; extend `CriticalWindowFrame`; new extraction fn; integrate into `observe()`; new test module entries)
- `crates/worldwake-sim/src/save_load.rs` (modify ONLY if `CriticalWindowFrame` is serialized — bump 115→116)

## Out of Scope

- The "Contested" cause for queue contention — explicitly dropped per spec D7 reassessment; queue contention is already covered by `wait_factor_permille` + `observe_wait`.
- A separate "Source acquisition success" record — out of scope; success is the absence of a failure record.
- Cross-window aggregation of source failures (e.g., "this agent fails at well A 5 times" trending) — out of scope; the per-frame collection is sufficient for the spec's emergent target.
- Observer CLI rendering of `SourceAcquisitionFailure` records — owned by ticket 008 (CLI player-POV).

## Acceptance Criteria

### Tests That Must Pass

1. New: `source_acquisition_failure_serialization_roundtrip` — bincode roundtrip with each cause/outcome combination.
2. New: `source_acquisition_failure_depleted_cause_from_expectation_failure_event` — seeded `EventTag::SourceExpectationFailure` produces a `cause: Depleted` record.
3. New: `source_acquisition_failure_quality_rejected_cause_from_quality_observed_event` — seeded `EventTag::ResourceSourceQualityObserved` with Muddy quality + intolerant agent produces `cause: QualityRejected`.
4. New: `source_acquisition_failure_outcome_traveled_to_fallback_when_subsequent_drink_at_different_source` — agent observes depleted source A, drinks at source B; outcome = `TraveledToFallback`.
5. New: `source_acquisition_failure_outcome_drank_anyway_when_subsequent_drink_at_same_source` — agent observes muddy source A, drinks at source A anyway; outcome = `DrankAnyway`.
6. New: `source_acquisition_failure_outcome_gave_up_when_no_subsequent_drink_in_window` — agent observes failure, never drinks in the critical window; outcome = `GaveUp`.
7. New: `clean_water_acquisition_produces_no_failure_record` — happy-path negative test.
8. Existing: `cargo test --workspace` passes.

### Invariants

1. `SourceAcquisitionFailure` records are derived from event-log + action-trace; no system writes them directly into authoritative state.
2. Cause is determined deterministically from event payload — same input always produces same cause.
3. Outcome is determined deterministically from action-trace ordering within the critical window.
4. The empty-collection case (no failures) is the dominant case for any healthy thirst window — the record is rare, not noisy.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/survival_forensics.rs` (test module extension) — 7 new focused tests covering record shape, cause classification, outcome reconstruction.

### Commands

1. `cargo test -p worldwake-ai source_acquisition_failure` — targeted forensic tests.
2. `cargo test -p worldwake-ai survival_forensics` — extractor integration tests.
3. `./scripts/verify.sh` — full workspace.
