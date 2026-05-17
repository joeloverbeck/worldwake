# S151TESRELROU-009: Diagnostics extension (per-topic map + flat-field removal + new counter)

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — replaces flat `source_reliability_changes: u64` with `source_reliability_changes_by_topic: BTreeMap<TopicScope, u64>`, adds new `route_preference_changes: u64`, aggregator + observer + fixture updates
**Deps**: archive/tickets/S151TESRELROU-001.md, S151TESRELROU-005

## Problem

S151's D10 extends `ScenarioDiagnosticsReport.belief` with two new diagnostics fields: a per-topic breakdown of testimony-reliability updates (`source_reliability_changes_by_topic: BTreeMap<TopicScope, u64>`) and a total route-preference update counter (`route_preference_changes: u64`). The existing flat `source_reliability_changes: u64` is removed per FND-28 — the spec's per-topic substrate makes it incomplete. The archived S144 spec at `archive/specs/S144-aggregate-scenario-diagnostics.md:141-142` explicitly fold-rejected the by-topic breakdown pending `TopicScope` landing; ticket 001 lands the substrate, this ticket lands the field.

## Assumption Reassessment (2026-05-17)

1. `ScenarioDiagnosticsReport` at `crates/worldwake-ai/src/scenario_diagnostics/mod.rs:12`; the `belief` sub-struct `BeliefMetrics` at lines 57-64 carries `stale_belief_actions`, `contradicted_belief_actions`, `source_reliability_changes`, `false_rumor_propagation_count`, `correction_latency`, `blocker_counts_by_scope`. The flat `source_reliability_changes` field is at line 60.
2. **Flat-field removal blast radius (per Step 2 spot-check (g))**: 8 sites across 4 files:
   - `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` — field def (line 60), test fixture (line 231)
   - `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` — private field (line 71), increment site (line 314), assign-out (line 447)
   - `crates/worldwake-cli/src/bin/observer.rs` — renderer (line 3779), test fixture (line 7348)
   - `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` — JSON field (line 5337)
3. **Golden fixture coupling**: `crates/worldwake-ai/tests/golden_scenario_diagnostics_harness/mod.rs:23` loads `expected-scenario-diagnostics.json` and asserts byte-stable output at line 122 (`scenario diagnostics fixture drifted; regenerate expected-scenario-diagnostics.json intentionally`). The fixture regeneration is part of this ticket's scope — replace the `source_reliability_changes: 0` JSON entry with `source_reliability_changes_by_topic: {}` and add `route_preference_changes: 0`.
4. **Aggregator data flow**: today's aggregator increments `source_reliability_changes` at `aggregator.rs:314` (reading what's likely a sibling decision-event field). With the per-topic breakdown, the aggregator must read `TestimonyTrustSummary.topic` from `GoalCommittedPayload.testimony_trust_context` (populated by ticket 006 via ticket 005's payload extension) and increment the matching `TopicScope` bucket. Verify the exact event source during implementation; the increment may also fire on `GoalSuppressedPayload.testimony_trust_context` for the suppression path. Likewise, `route_preference_changes` increments on `GoalCommittedPayload.route_preference_context` non-empty.
5. **Naming convention**: per Step 2 spot-check (g) discussion — chose `route_preference_changes` (no suffix, plural noun) to mirror existing `source_reliability_changes`. Avoid `_count` suffix for consistency with the sibling field.

## Architecture Check

1. Per FND-28: no shim retaining the flat `source_reliability_changes` alongside the new map. The flat field is removed and consumers migrate to the map. This is the explicit FND-28-driven scope extension flagged in the reassessment's pre-apply table.
2. Per FND-3: per-topic breakdown is concrete state — each bucket count is attributable to a specific `TopicScope` and an enumerable set of update events.
3. Per CLAUDE.md Determinism: `BTreeMap<TopicScope, u64>` preserves iteration order; bincode round-trip is deterministic.
4. Per FND-29: per-topic visibility makes scenario diagnostics legible to debugging — "which topic categories did this scenario stress-test the witness reliability surface for?" becomes answerable.
5. Naming pairing (`source_reliability_changes_by_topic` mirrors existing `source_reliability_changes`; `route_preference_changes` follows the same suffix convention) keeps the diagnostics surface coherent.

## Verification Layers

1. Aggregator increment correctness → focused unit test in `aggregator.rs#[cfg(test)]` — feed the aggregator with a `GoalCommittedPayload` containing two `TestimonyTrustSummary` entries (different topics) and assert both bucket increments fire.
2. Flat-field removal → grep workspace for `source_reliability_changes` post-edit; only the new `_by_topic` form remains (apart from intentional FND-28 documentation).
3. Fixture round-trip → `golden_scenario_diagnostics_harness` passes with the regenerated `expected-scenario-diagnostics.json`.
4. Observer rendering → unit test asserting the new map renders as a nested topic-by-topic list in the diagnostics report.

## What to Change

### 1. Replace flat field with map and add new counter (`crates/worldwake-ai/src/scenario_diagnostics/mod.rs:57-64`)

```rust
pub struct BeliefMetrics {
    pub stale_belief_actions: u64,
    pub contradicted_belief_actions: u64,
    pub source_reliability_changes_by_topic: BTreeMap<TopicScope, u64>,  // replaces flat source_reliability_changes
    pub route_preference_changes: u64,                                    // new
    pub false_rumor_propagation_count: u64,
    pub correction_latency: PercentileBucket,
    pub blocker_counts_by_scope: BTreeMap<BlockerScopeVariantId, u64>,
}
```

Update the test fixture at `mod.rs:231` to use the new field names.

### 2. Update aggregator (`crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`)

- Line 71: replace `source_reliability_changes: u64` with `source_reliability_changes_by_topic: BTreeMap<TopicScope, u64>` and add `route_preference_changes: u64`.
- Line 314: change `self.source_reliability_changes += 1` to iterate the relevant decision-event payload's `testimony_trust_context` and increment per-topic.
- Add a new increment site (or extend the existing one) for `route_preference_changes` when a `GoalCommittedPayload.route_preference_context` is non-empty.
- Line 447: update the `BeliefMetrics` construction to assign both new fields.

The exact decision-event read path depends on the aggregator's current event-iteration shape — read `aggregator.rs` end-to-end during implementation to determine the cleanest integration site (likely where `GoalCommitted` payloads are already processed).

### 3. Update observer renderer (`crates/worldwake-cli/src/bin/observer.rs`)

Line 3779: replace the flat `Source reliability changes: N` rendering with a nested per-topic list:

```
- **Source reliability changes (by topic)**:
  - RouteHazard: 12
  - ResourceAvailability: 7
  - EntityWhereabouts: 3
  - (other topics with 0 counts omitted)
- **Route preference changes**: 14
```

Update the renderer's accompanying test fixture (line 7348) to match the new field shape.

### 4. Regenerate `expected-scenario-diagnostics.json`

Run the diagnostics harness in regeneration mode (the exact command lives in the harness — likely an `UPDATE=1` env var or a `--write` flag; check `golden_scenario_diagnostics_harness/mod.rs` during implementation). Verify the regenerated JSON includes both new fields with the correct shape and zero/empty initial values.

## Files to Touch

- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (modify — field replacement, new field, test fixture)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — private field, increment logic, output assignment)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — renderer at line 3779, test fixture at line 7348)
- `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (modify — regenerate via the harness)

## Out of Scope

- Decision-event payload extensions that feed the per-topic data — ticket 005
- Observation hook that emits the events the aggregator consumes — ticket 006
- Consumer ranking damping that produces `GoalSuppressedPayload` with `testimony_trust_context` — ticket 007
- `SAVE_FORMAT_VERSION` bump — ticket 010 (the diagnostics report is computed at run-time, not part of the engine save format, so this ticket itself doesn't require a bump)

## Acceptance Criteria

### Tests That Must Pass

1. `BeliefMetrics` no longer carries `source_reliability_changes: u64`; only `source_reliability_changes_by_topic` and `route_preference_changes`.
2. Aggregator increments the correct per-topic bucket when processing a `GoalCommittedPayload` with non-empty `testimony_trust_context`.
3. Aggregator increments `route_preference_changes` when processing a `GoalCommittedPayload` with non-empty `route_preference_context`.
4. `golden_scenario_diagnostics_harness` passes with the regenerated `expected-scenario-diagnostics.json`.
5. Observer renderer produces a nested per-topic list for the new map field.
6. Existing suite: `cargo test --workspace`.

### Invariants

1. No `source_reliability_changes` field remains in the codebase (except in FND-28 documentation noting the removal).
2. `BTreeMap<TopicScope, u64>` iteration order is deterministic (per CLAUDE.md Determinism invariant).
3. Diagnostics output is byte-stable across reruns on a fixed scenario seed.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs#[cfg(test)]` — new test for per-topic incrementing; new test for route-preference incrementing.
2. `crates/worldwake-ai/src/scenario_diagnostics/mod.rs#[cfg(test)]` — update existing test fixture at line 231.
3. `crates/worldwake-ai/tests/golden_scenario_diagnostics_harness/mod.rs` — verify passes against regenerated fixture.

### Commands

1. `cargo test -p worldwake-ai scenario_diagnostics`
2. `cargo test -p worldwake-ai golden_scenario_diagnostics`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh`
