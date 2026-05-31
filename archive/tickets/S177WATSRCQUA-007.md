# S177WATSRCQUA-007: Survival forensics `SourceAcquisitionFailure` record

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai/survival_forensics` (new `SourceAcquisitionFailure` derived record + `SourceFailureCause`/`SourceFailureOutcome` enums; extension to `SurvivalForensicExtractor.observe()`)
**Deps**: `archive/tickets/S177WATSRCQUA-004.md`

## Problem

The spec's D7 deliverable adds a derived forensic record to `SurvivalForensicExtractor` that surfaces when an agent's water-acquisition attempt failed (depleted source or quality-driven rejection). This complements the existing `DegradedSelfCareOpportunity` record in `crates/worldwake-ai/src/survival_forensics.rs`, which covers basin/latrine failures. Without this record, the critical-window forensics view cannot explain "the agent's thirst critical window includes 3 attempts at depleted/muddy sources" — debuggability (FND-29A) is incomplete for the new quality axis.

## Assumption Reassessment (2026-05-31)

1. `SurvivalForensicExtractor` in `crates/worldwake-ai/src/survival_forensics.rs` accumulates `CriticalWindowFrame`s per need during critical windows and flushes `CriticalWindowReport` on need recovery. The `observe()` method builds each frame.
2. Existing forensic record `DegradedSelfCareOpportunity` in `crates/worldwake-ai/src/survival_forensics.rs` has shape `{ tick, facility, cause: DegradedSelfCareCause, outcome: DegradedSelfCareOutcome }`. Causes are `BasinTooDirty, BasinDry, LatrineFull`; outcomes are `WildernessRelief, Cleaned, Queued, DidNothing`. The extraction fn `degraded_self_care_opportunities()` reads action-trace events at the tick.
3. The new `SourceAcquisitionFailure` record's surface is derived from: (a) `EventTag::SourceExpectationFailure` emissions (already live — see `production_actions.rs:1228` writing `record_failed_source_attempt` paired with the event emission in `agent_tick/mod.rs:1051-1052`); (b) `EventTag::ResourceSourceQualityObserved` emissions (added by ticket 004) where the observed quality matches a "rejection" condition. The extractor reads these from the action-trace / event-log snapshot.
4. Per the reassessed spec D7, the cause set is `Depleted` and `QualityRejected` — the speculative "Contested" cause was dropped because queue contention is already surfaced via `wait_factor_permille` in source_composite_rank and `observe_wait` on `ReliabilityRecord`. Adding a third cause for contention here would duplicate the existing queue substrate.
5. The outcome set per spec D7 is `DrankAnyway, TraveledToFallback, GaveUp`. The landed implementation reconstructs same-tick outcomes from action-trace events and updates earlier `GaveUp` records when later ticks in the same critical window show travel or drinking. The live action trace does not carry drink/harvest source identity, so the outcome is behavioral (`drank`, `traveled`, or no response), while the failure cause remains source-specific from the event payload.
6. Shared abstraction boundary: the `CriticalWindowFrame`'s `source_acquisition_failures: Vec<SourceAcquisitionFailure>` collection. The frame already carries `local_authoritative_summary: LocalSurvivalStateSummary` and other forensic accumulators per S120's archived spec — adding this collection follows the same pattern.
7. Adjacent contradictions: the existing `degraded_self_care_opportunities()` extraction is event-driven (reads action-trace at the tick); the new extraction is event-driven (reads `SourceExpectationFailure` + `ResourceSourceQualityObserved` at the tick). Symmetry preserved.
8. SAVE_FORMAT_VERSION: `CriticalWindowFrame` and `CriticalWindowReport` remain absent from `crates/worldwake-sim/src/save_load.rs` and `crates/worldwake-core/src/delta.rs`; the new collection is derived forensic state, not serialized authoritative state. No save-format bump was required.
9. Reassessment correction: `SurvivalForensicExtractor::observe()` did not previously receive the event log, so the event-derived failure causes were unreachable through the drafted helper signature. The landed seam adds `event_log: &EventLog` as a read-only extractor input and updates observer/golden-harness callers.

## Architecture Check

1. Derived forensic record (vs. authoritative state) follows FND-3 — the failure facts are derived from event-log + action-trace, not stored authority. FND-27: derived summaries are caches over source state.
2. Cause set `{Depleted, QualityRejected}` (vs. wider speculative set) follows YAGNI and the spec's reassessment-driven scope narrowing — adding causes for unproven scenarios is dead surface.
3. Outcome reconstruction from subsequent action-trace events (vs. recording outcome at the time of failure) — the agent's chosen response to the failure is observable from what they did next, not from the failure event itself. FND-29A inspectability: the causal chain is reconstructable.

## Verified Layers

1. `SourceAcquisitionFailure` record fields compile and roundtrip — `source_acquisition_failure_serialization_roundtrip`.
2. Extraction fires for `SourceExpectationFailure` event — `source_acquisition_failure_depleted_cause_from_expectation_failure_event` asserts `cause: Depleted`.
3. Extraction fires for non-clean `ResourceSourceQualityObserved` — `source_acquisition_failure_quality_rejected_cause_from_quality_observed_event` asserts `cause: QualityRejected`.
4. Outcome reconstruction updates a prior record from later action trace in the same critical window — `source_acquisition_failure_updates_prior_gave_up_when_window_later_travels`.
5. Clean-water observation produces no failure record — `source_acquisition_failure_travel_and_clean_negative_cases`.

## Landed Changes

### 1. Added `SourceAcquisitionFailure` record + enums

`crates/worldwake-ai/src/survival_forensics.rs` (near the existing `DegradedSelfCareOpportunity` types):

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

### 2. Extended `CriticalWindowFrame`

Added `source_acquisition_failures: Vec<SourceAcquisitionFailure>` to `CriticalWindowFrame` with `#[serde(default)]`.

### 3. Added extraction fn `source_acquisition_failures()`

Analogous to `degraded_self_care_opportunities()`. Reads action trace + event log for the tick window, matches on `EventTag::SourceExpectationFailure` and `EventTag::ResourceSourceQualityObserved` payloads, classifies the cause, and reconstructs outcomes from same-tick or subsequent action-trace events in the critical window:

```rust
fn source_acquisition_failures(
    tick: Tick,
    action_trace_snapshot: &ActionTraceSnapshot,
    event_log: &EventLog,
    agent: EntityId,
) -> Vec<SourceAcquisitionFailure> {
    // Iterates events at the tick, filters for SourceExpectationFailure +
    // ResourceSourceQualityObserved matching agent, classifies cause, and
    // combines with same-window action-trace response.
}
```

The implementation follows the existing `degraded_self_care_opportunities` extractor shape while adding read-only event-log access for event-derived causes.

### 4. Integrated into `SurvivalForensicExtractor.observe()`

At the frame-building site, `observe()` now receives a read-only `EventLog`, calls `source_acquisition_failures(...)`, attaches records to the frame, and updates earlier `GaveUp` records when later ticks in the same window show a response.

### 5. Save format unchanged

`CriticalWindowFrame` and `CriticalWindowReport` are not serialized into authoritative `SimulationState`, so no save-format bump landed.

## Landed Files

- `crates/worldwake-ai/src/survival_forensics.rs` — new record + enums; `CriticalWindowFrame` field; event-log-backed extraction; window-level outcome update; focused tests.
- `crates/worldwake-ai/src/lib.rs` — re-exported the new forensic types.
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` — passed the harness event log into `observe()`.
- `crates/worldwake-ai/tests/integration/forensic_determinism.rs` — passed an empty event log into synthetic extractor tests.
- `crates/worldwake-ai/tests/integration/forensic_sleep_progress_barrier.rs` — passed an empty event log into synthetic extractor tests.
- `crates/worldwake-ai/tests/integration/forensic_wash_vs_water_competition.rs` — passed an empty event log into synthetic extractor tests.
- `crates/worldwake-cli/src/bin/observer.rs` — passed the simulation event log into observer critical-window extraction.
- No change: `crates/worldwake-sim/src/save_load.rs`; no save-format bump was needed.

## Out of Scope

- The "Contested" cause for queue contention — explicitly dropped per spec D7 reassessment; queue contention is already covered by `wait_factor_permille` + `observe_wait`.
- A separate "Source acquisition success" record — out of scope; success is the absence of a failure record.
- Cross-window aggregation of source failures (e.g., "this agent fails at well A 5 times" trending) — out of scope; the per-frame collection is sufficient for the spec's emergent target.
- Observer CLI rendering of `SourceAcquisitionFailure` records — owned by ticket 008 (CLI player-POV).

## Acceptance Result

### Tests Passed

1. `source_acquisition_failure_serialization_roundtrip` — bincode roundtrip with each cause/outcome combination.
2. `source_acquisition_failure_depleted_cause_from_expectation_failure_event` — seeded `EventTag::SourceExpectationFailure` produces a `cause: Depleted` record.
3. `source_acquisition_failure_quality_rejected_cause_from_quality_observed_event` — seeded `EventTag::ResourceSourceQualityObserved` with `Muddy` quality produces a `cause: QualityRejected` record.
4. `source_acquisition_failure_updates_prior_gave_up_when_window_later_travels` — agent observes a depleted source, later starts travel during the same critical window, and the earlier record updates to `TraveledToFallback`.
5. `source_acquisition_failure_quality_rejected_cause_from_quality_observed_event` also proves `DrankAnyway` for a same-tick committed `drink`.
6. `source_acquisition_failure_depleted_cause_from_expectation_failure_event` proves default `GaveUp` when no response appears.
7. `source_acquisition_failure_travel_and_clean_negative_cases` — clean quality observation produces no failure record while non-clean quality does.
8. Existing affected crate suite: `cargo test -p worldwake-ai` passed.

### Invariants

1. `SourceAcquisitionFailure` records are derived from event-log + action-trace; no system writes them directly into authoritative state.
2. Cause is determined deterministically from event payload — same input always produces same cause.
3. Outcome is determined deterministically from action-trace ordering within the critical window.
4. The empty-collection case (no failures) is the dominant case for any healthy thirst window — the record is rare, not noisy.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/survival_forensics.rs` — 5 new focused tests covering record shape, cause classification, outcome reconstruction, later-window outcome update, and clean-water negative behavior.

### Commands Run

1. `cargo test -p worldwake-ai source_acquisition_failure`
2. `cargo test -p worldwake-ai survival_forensics`
3. `cargo test -p worldwake-ai --no-run`
4. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-05-31.

- Added the derived `SourceAcquisitionFailure` forensic record, `SourceFailureCause`, and `SourceFailureOutcome`.
- Extended `CriticalWindowFrame` with `source_acquisition_failures` and re-exported the new public types.
- Made `SurvivalForensicExtractor::observe()` read the append-only event log so source-failure causes remain derived from existing decision/event payloads instead of becoming authoritative state.
- Updated observer, golden harness, and synthetic integration callers to provide the event log.
- Left authoritative save format unchanged because the forensic report/frame types are not part of `SimulationState`.

## Deviations

- The drafted helper expected source-specific drink/harvest outcome reconstruction from action trace. The live action trace does not carry source identity for those actions, so this ticket records source-specific causes from event payloads and behavioral outcomes from action names (`drink`, `travel`, or no response). The window-level update still reconstructs later action-trace response within the same critical window.
- The drafted `./scripts/verify.sh` row is waived for this per-ticket harness iteration; the full `implement-spec-tickets` final branch phase owns the pre-push `./scripts/verify.sh` gate.

## Verification Result

- Passed `cargo test -p worldwake-ai source_acquisition_failure`
- Passed `cargo test -p worldwake-ai survival_forensics`
- Passed `cargo test -p worldwake-ai --no-run`
- Passed `cargo test -p worldwake-ai`
- Waived `./scripts/verify.sh` for this ticket iteration because the harness final branch phase runs it before push.
