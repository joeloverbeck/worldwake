# S150CROGOABLO-005: S144 per-scope blocker diagnostics

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `ScenarioDiagnosticsReport.belief` extension; new `BlockerScopeVariantId` enum; aggregator extension
**Deps**: archive/tickets/S150CROGOABLO-002.md

## Problem

S144 (archived) provides the `ScenarioDiagnosticsReport` substrate that aggregates per-scenario behavioral metrics, but its `BeliefMetrics` section does not currently track blocker counts by scope. After ticket 002, blockers carry typed scope information (`Exact`, `RouteSegment`, `Counterparty`); operators reviewing a scenario's diagnostics output cannot answer "how many cross-goal blockers fired versus how many goal-specific ones?" without manually walking the event log. S150 D8 specifies a per-scope histogram on `BeliefMetrics` so the aggregate scope distribution is inspectable through the existing S144 reporting framework.

## Assumption Reassessment (2026-05-17)

1. `ScenarioDiagnosticsReport` lives at `crates/worldwake-ai/src/scenario_diagnostics/mod.rs:12-20` with subsections including `belief: BeliefMetrics` (line 17). `BeliefMetrics` is defined at lines 57-63 with 5 current fields. The aggregator that populates `BeliefMetrics` lives at `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`.
2. Spec source: `specs/S150-cross-goal-blocker-scoping.md` D8 (`blocker_counts_by_scope: BTreeMap<BlockerScopeVariantId, u64>` with `BlockerScopeVariantId` defined alongside the existing `CandidateSuppressionCategory` precedent at `scenario_diagnostics/mod.rs:90-108`).
3. Shared abstraction boundary: the diagnostics aggregator reads `BlockerRecorded` events from the event log (the `scope: BlockerScope` field added by ticket 002), projects each scope to its `BlockerScopeVariantId` discriminant, and accumulates the histogram. The histogram key (`BlockerScopeVariantId`) is the aggregation-key surface; the `BlockerScope` source enum carries payload-bearing variants that would fragment the histogram if used directly (per `references/codebase-validation.md` 3.2 "Aggregation-key fidelity" rule).
4. Existing tests in target module: `crates/worldwake-ai/src/scenario_diagnostics/mod.rs::scenario_diagnostics_report_round_trips_through_serde` (line 121), `scenario_diagnostics_report_round_trips_through_json_for_string_keyed_values` (line 131), `candidate_suppression_category_is_ordered_and_serde_ready` (line 141). The first two have a `populated_report` fixture at line 163 that must gain a non-empty `blocker_counts_by_scope` value. The third is the precedent pattern for testing the new `BlockerScopeVariantId` enum.
5. Adjacent contradictions: none. The new field is additive on `BeliefMetrics`; the new enum is sited alongside `CandidateSuppressionCategory` and follows its exact derive set + ordering pattern.

## Architecture Check

1. **Aggregation-key fidelity**: `BlockerScopeVariantId` is a payload-free enum (Exact/RouteSegment/Counterparty) used as the `BTreeMap` key. Using the full `BlockerScope` as the key would fragment the histogram by payload value (every distinct `BlockerScope::RouteSegment(seg)` would be its own bucket). The variant-id enum collapses the buckets to the discriminant axis.
2. **Mirrors existing precedent**: `BlockerScopeVariantId` shape, derives, ordering, and placement mirror `CandidateSuppressionCategory` (scenario_diagnostics/mod.rs:90-108) — same derive set (`Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`), same module location. Consistent with the project's existing aggregation-key convention.
3. **Append-only-event-log compatible**: The aggregator reads `BlockerRecorded` events that already exist in the append-only log (no new event tag, no new authoritative state); the histogram is a derived view per FND-27.

## Verification Layers

1. `BlockerScopeVariantId` trait-bound regression — focused unit test (mirroring `candidate_suppression_category_is_ordered_and_serde_ready` at line 141) proving the new enum is ordered, serde-ready, and the variants enumerate distinctly.
2. Aggregator histogram correctness — focused unit test in `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` `#[cfg(test)]`: feed the aggregator a synthetic event sequence containing N `BlockerRecorded` events with each scope variant, assert the resulting `blocker_counts_by_scope` map values.
3. Serde roundtrip — extend `scenario_diagnostics_report_round_trips_through_serde` to populate `blocker_counts_by_scope` with each variant; assert the bincode roundtrip preserves the map.
4. JSON serialization roundtrip — extend `scenario_diagnostics_report_round_trips_through_json_for_string_keyed_values` for the same.

## What to Change

### 1. Add `BlockerScopeVariantId` enum

In `crates/worldwake-ai/src/scenario_diagnostics/mod.rs`, alongside the existing `CandidateSuppressionCategory` enum (lines 90-108):

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BlockerScopeVariantId {
    Exact,
    RouteSegment,
    Counterparty,
}
```

### 2. Extend `BeliefMetrics` with the per-scope histogram

```rust
pub struct BeliefMetrics {
    pub stale_belief_actions: u64,
    pub contradicted_belief_actions: u64,
    pub source_reliability_changes: u64,
    pub false_rumor_propagation_count: u64,
    pub correction_latency: PercentileBucket,
    pub blocker_counts_by_scope: BTreeMap<BlockerScopeVariantId, u64>,  // NEW
}
```

Update the `populated_report` fixture at line 163 to populate `blocker_counts_by_scope` with non-empty values for each variant (so the serde roundtrip tests exercise the new field).

### 3. Extend the aggregator to populate `blocker_counts_by_scope`

In `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`:

- Walk `BlockerRecorded` events from the event log (the event tag exists from S110; the `scope: BlockerScope` field exists from ticket 002).
- For each event, project `scope` to its `BlockerScopeVariantId`:
  ```rust
  let variant_id = match payload.scope {
      BlockerScope::Exact(_) => BlockerScopeVariantId::Exact,
      BlockerScope::RouteSegment(_) => BlockerScopeVariantId::RouteSegment,
      BlockerScope::Counterparty(_) => BlockerScopeVariantId::Counterparty,
  };
  *blocker_counts_by_scope.entry(variant_id).or_insert(0) += 1;
  ```
- Initialize the map empty (no need to pre-seed zero entries — readers can interpret a missing key as zero).

### 4. Add focused trait-bound and aggregator tests

- `blocker_scope_variant_id_is_ordered_and_serde_ready` (new, mirrors `candidate_suppression_category_is_ordered_and_serde_ready`): construct a `BTreeMap<BlockerScopeVariantId, u64>` with all three variants, serialize through JSON, assert ordering and round-trip equality.
- `aggregator_populates_blocker_counts_by_scope` (new): feed a synthetic event sequence with 3 Exact / 2 RouteSegment / 1 Counterparty `BlockerRecorded` events, assert the histogram values.

## Files to Touch

- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (modify) — `BlockerScopeVariantId` enum + `BeliefMetrics` field extension + fixture update + new bound-test
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify) — aggregator walks BlockerRecorded events + projects scope variant + new aggregator-test

## Out of Scope

- **`BlockerScope` enum and `RouteSegment` newtype** — landed in ticket 002.
- **`scope: BlockerScope` field on `BlockerRecordedPayload`** — landed in ticket 002.
- **Observer Section 13 rendering of the new histogram** — Section 13 already renders `BeliefMetrics` via the existing `render_belief_metrics`-style helper (search `observer.rs` Section 13 rendering pipeline at line 3696); the new field renders through the existing framework. If the new field needs a distinct rendering treatment beyond the existing pattern, that's a follow-up Small ticket; for this ticket the default rendering through the existing framework is sufficient.
- **Per-blocker provenance reporting in diagnostics** — `source_event: EventId` from ticket 002 is per-blocker live state, not aggregate diagnostics. If aggregate `source_event` reporting is wanted later, it's a separate spec.

## Acceptance Criteria

### Tests That Must Pass

1. `blocker_scope_variant_id_is_ordered_and_serde_ready` (new) — variant ordering and serde round-trip.
2. `aggregator_populates_blocker_counts_by_scope` (new) — histogram values match input event sequence.
3. `scenario_diagnostics_report_round_trips_through_serde` (extended) — full report with non-empty `blocker_counts_by_scope` round-trips through bincode.
4. `scenario_diagnostics_report_round_trips_through_json_for_string_keyed_values` (extended) — same through JSON.
5. Workspace: `cargo test -p worldwake-ai --lib scenario_diagnostics` clean.

### Invariants

1. `BlockerScopeVariantId` is payload-free — the histogram aggregates by discriminant axis, not by payload.
2. The aggregator's histogram count equals the number of `BlockerRecorded` events in the input event log (sum of `blocker_counts_by_scope.values()` == event count).
3. `BlockerScopeVariantId` continues to satisfy `Copy + Hash + Ord + Serialize + Deserialize` (parity with `CandidateSuppressionCategory`).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` — `blocker_scope_variant_id_is_ordered_and_serde_ready` (new); `populated_report` fixture extension; `scenario_diagnostics_report_round_trips_through_serde` / `_json_*` assertions extended.
2. `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` — `aggregator_populates_blocker_counts_by_scope` (new).

### Commands

1. `cargo test -p worldwake-ai --lib scenario_diagnostics`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `./scripts/verify.sh` for the full pre-PR gate.
