# S133SOUCOMTIE-004: Decision-trace surfacing for SourceCompositeRank

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — decision-trace formatter and per-candidate trace block extension
**Deps**: archive/tickets/S133SOUCOMTIE-003.md

## Problem

After ticket 003, `AgendaEntry.source_composite` is populated and the comparator attributes flips to `RankedGoalComparisonDimension::SourceComposite`. Without trace text the per-factor breakdown is invisible to debug trace consumers. FND-29 (debuggability is a product feature) requires the trace to expose the trust/wait/capacity factors that drove the rank flip; FND-29A keeps that data on the existing decision-trace carrier (no new event tag).

## Assumption Reassessment (2026-05-03)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The per-candidate trace block already invokes `format_source_reliability_discount_summary` at two sites: `crates/worldwake-ai/src/decision_trace.rs:175-176` (planning summary) and `:1343-1349` (debug dump ranked output). Both consume `RankedGoalSummary.source_reliability_discount`. The new helper `format_source_composite_summary` mirrors that path exactly and feeds off the new `RankedGoalSummary.source_composite` populated by ticket 003. Existing focused coverage: the summary substring assertion at `decision_trace.rs:3839-3850` (asserts `source_reliability=entity=`, `commodity=Bread`, `failure=500`, etc.).
2. The spec D5 dictates the format: `, source_composite=entity={} commodity={:?} trust={} wait={} cap={} composite={}`. The existing format-helper convention places a leading `", "` and uses positional `{}`/`{:?}` — verified at `decision_trace.rs:1957-1971`.
3. Shared abstraction boundary under audit: `RankedGoalSummary` (the trace-side projection of `AgendaEntry`) and the per-candidate summary aggregator. Live reassessment corrected the observer claim: CLI observer Section 3 calls `decision_payload_summary(payload)` over `DecisionEventPayload`, and `GoalCommittedPayload` only carries `rejected_alternatives` count-level data, not per-candidate `RankedGoalSummary` records. The owned D5 surface is therefore `DecisionOutcome::summary()` plus `DecisionTraceSink::dump_agent()` per-candidate ranked output; no observer-side renderer or core decision-event payload migration landed in this ticket.

## Architecture Check

1. Adding a focused `format_source_composite_summary` mirrors the existing pattern (`format_source_reliability_discount_summary`, `format_competition_discount_summary`); the new helper is a pure projection of `SourceCompositeRank`, no new dependencies. Alternatives considered: (i) merging the composite line into `format_source_reliability_discount_summary` — rejected because the failure-ratio discount and the composite are conceptually distinct (motive-discount vs intra-commodity tiebreaker), and the spec calls them out as two separate trace surfaces; (ii) adding observer-side rendering — rejected because the live observer decision-event section does not carry per-candidate `RankedGoalSummary` records and a payload migration is outside D5.
2. No backward-compat shim. The new line appears alongside the existing `source_reliability=` line when both apply.

## Verification Layers

1. New format helper output → focused unit test in `decision_trace.rs::tests` asserting `source_composite=entity=`, `trust=`, `wait=`, `cap=`, `composite=` substrings on a populated sample.
2. Combined trace block (both reliability discount and composite present on the same candidate) → existing summary assertion test at `decision_trace.rs:3841` extended to additionally cover `source_composite=` substring.
3. CLI observer regression → existing `observer_decision_history` test remains green; it proves the pre-existing event-payload report is unaffected, not that Section 3 renders per-candidate composite fields.
6. Single-layer (decision-trace formatter) ticket; no cross-system or authoritative state implication.

## What to Change

### 1. Add `format_source_composite_summary`

In `crates/worldwake-ai/src/decision_trace.rs`, near `format_source_reliability_discount_summary` (line 1957):

```rust
fn format_source_composite_summary(rank: &SourceCompositeRank) -> String {
    format!(
        ", source_composite=entity={} commodity={:?} trust={} wait={} cap={} composite={}",
        rank.source_entity,
        rank.commodity,
        rank.trust_factor_permille,
        rank.wait_factor_permille,
        rank.capacity_factor_permille,
        rank.composite_permille,
    )
}
```

### 2. Wire into both per-candidate trace block sites

`decision_trace.rs:175-176`:

```rust
let source_composite_summary = ranked
    .as_ref()
    .and_then(|summary| summary.source_composite.as_ref())
    .map_or_else(String::new, format_source_composite_summary);
```

…and append `source_composite_summary` to the candidate summary string alongside the existing `source_reliability_discount_summary` and `competition_discount_summary`.

`decision_trace.rs:1343-1349`:

```rust
let source_composite_summary = ranked
    .source_composite
    .as_ref()
    .map_or_else(String::new, format_source_composite_summary);
```

…and append in the same format-aggregation pattern.

### 3. Add focused test fixture and assertion

Add `sample_source_composite_rank()` near `sample_source_reliability_discount` (line 2461). Add a focused test asserting `format_source_composite_summary` output contains all expected substrings.

### 4. Extend existing summary assertion test

In `decision_trace.rs:3839-3850`, when the test fixture includes a populated `source_composite`, additionally assert:

```rust
assert!(summary.contains("source_composite=entity="));
assert!(summary.contains("trust="));
assert!(summary.contains("composite="));
```

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — new helper, two trace-block call sites, sample fixture, two focused assertions)

## Out of Scope

- Comparator semantics (ticket 003).
- Vestigial-field removal on `SourceReliabilityDiscount` (ticket 005).
- Golden E2E (ticket 006).
- Observer-side renderer changes and core decision-event payload widening — live reassessment showed Section 3 does not carry per-candidate `RankedGoalSummary` data.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test asserting `format_source_composite_summary` output contains `source_composite=entity=`, `trust=`, `wait=`, `cap=`, `composite=`.
2. Existing summary assertion test at `decision_trace.rs:3841` extended to additionally assert the composite line; still passes the failure-ratio reliability assertions.
3. Existing `crates/worldwake-cli/tests/observer_decision_history.rs` remains green as a no-regression check for the unchanged decision-event report surface.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. The composite trace line and the reliability-discount trace line may coexist on the same candidate when both apply (Design Goal 8).
2. Trace text is emitted only via the existing decision-trace summary/debug-dump path; no new event tag is added (FND-29A).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs::tests::format_source_composite_summary_emits_factor_substrings` — new.
2. `crates/worldwake-ai/src/decision_trace.rs::tests` extended summary assertion at line 3841 — modified.

### Commands

1. `cargo test -p worldwake-ai decision_trace::tests` (focused).
2. `cargo test -p worldwake-cli --test observer_decision_history survival_baseline_decision_history_section_matches_golden -- --exact` (unchanged observer decision-event report surface remains green).
3. `cargo test --workspace` (full).

## Outcome

Completed on 2026-05-04.

- Added `format_source_composite_summary` in `crates/worldwake-ai/src/decision_trace.rs`.
- Threaded `source_composite` formatting into selected planning summaries and the per-candidate `DecisionTraceSink::dump_agent()` ranked output alongside the existing source-reliability and competition suffixes.
- Added `sample_source_composite_rank()` plus focused coverage for the standalone composite formatter and the combined selected-summary path where source reliability and source composite coexist.
- Truth-synced `specs/S133-source-composite-tiebreaker.md` D5 wording to the live renderer boundary.

## Deviations

- Reassessment corrected the drafted observer claim. CLI observer Section 3 summarizes `DecisionEventPayload` rows and does not have access to per-candidate `RankedGoalSummary.source_composite`; this ticket did not widen the core decision-event payload or observer renderer. The implemented surface is the decision-trace summary/debug-dump path.
- The drafted command `cargo test -p worldwake-cli observer_decision_history` compiled the intended integration-test binary but executed zero tests. The truthful observer proof was rebound to the exact live integration-test selector.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib decision_trace::tests -- --list`.
- Passed `cargo test -p worldwake-ai --lib decision_trace::tests::format_source_composite_summary_emits_factor_substrings -- --exact`.
- Passed `cargo test -p worldwake-ai --lib decision_trace::tests::summary_planning_includes_selected_source_reliability_discount -- --exact`.
- Passed `cargo test -p worldwake-ai --lib decision_trace::tests`.
- Passed `cargo test -p worldwake-cli --test observer_decision_history -- --list`.
- Passed `cargo test -p worldwake-cli --test observer_decision_history survival_baseline_decision_history_section_matches_golden -- --exact`.
- Passed `cargo test --workspace`.
