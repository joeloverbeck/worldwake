# S144AGGSCEDIA-003: PlanningSnapshot cache counters and decision-trace surfacing

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — read-only logical counters on `PlanningSnapshot` cache accesses; new carrier field on `AgentDecisionTrace`. No planning-behavior change.
**Deps**: None — foundation ticket (S144 D8)

## Problem

S144's `PerformanceMetrics` needs deterministic logical cache hit/miss/invalidation counts. `PlanningSnapshot` holds precomputed travel/cost caches but counts no logical access events, and the decision trace carries no surface for them. Without this instrumentation, the aggregator (ticket 005) cannot populate `PerformanceMetrics.cache_*` fields.

## Assumption Reassessment (2026-05-14)

1. `PlanningSnapshot` is at `crates/worldwake-ai/src/planning_snapshot.rs:432`; its cache surface is two `DistanceMatrix` fields (`shortest_travel_ticks`, `perceived_travel_costs` at :465-466), accessed through `DistanceMatrix::get` (:381, returns `Option<u32>`) via the public methods `min_travel_ticks` (:811), `min_travel_ticks_to_any` (:822), `min_perceived_travel_cost_to_any` (:837), `direct_perceived_travel_cost` (:852), `direct_perceived_travel_breakdown` (:858). Existing inline tests exercising this surface (`#[cfg(test)]` at :1367): `min_travel_ticks_self_is_zero` (:2935), `min_travel_ticks_direct_adjacent` (:2942), `snapshot_filter_excludes_items_for_travel_only_goal` (:1855) — these must continue to pass; the instrumentation is purely additive and changes no return values.
2. S144 spec D8 (`specs/S144-aggregate-scenario-diagnostics.md`) specifies read-only `u64` logical counters on the snapshot caches surfaced through the decision trace, explicitly framed as a derived read-model addition (FND-27) with no planning-behavior change. `AgentDecisionTrace` (`decision_trace.rs:94`, derives `Clone, Debug`) already carries an analogous optional load carrier `opportunity_compiler_load: Option<OpportunityCompilerLoad>` (:99) — the new counter carrier follows that precedent.
3. Mixed-layer data contract under audit: `AgentDecisionTrace` gains a new carrier field. `AgentDecisionTrace` has 23 construction sites across 7 files (`agent_tick/mod.rs`, `decision_trace.rs`, `survival_forensics.rs`, `bin/observer.rs`, `worldwake-visualizer/src/trace_buffers.rs`, `tests/golden_harness/timeline.rs`, `tests/golden_harness/survival_forensics_assertions.rs`); `AgentDecisionTrace` derives no `Default`, so every site must add the new field explicitly (as `None` for sites that produce no planning snapshot).
4. Adjacent-contradiction classification (required consequence of D8, not a separate bug): `DistanceMatrix` is a *precomputed* matrix, not a populate-on-demand cache — there is no natural "invalidation" event for it, and every `get` is a hit-or-absent rather than a hit-or-recompute-miss. The exact mapping of `hit_count`/`miss_count`/`invalidation_count` onto this precomputed surface must be pinned during implementation: a `Some` result from `DistanceMatrix::get` is the hit, a `None` is the miss, and `invalidation_count` is `0` for a precomputed matrix unless a populate-on-demand cache is also identified and instrumented. Record the chosen mapping in What to Change section 1 before editing.

## Architecture Check

1. Counting logical cache accesses inside `PlanningSnapshot` and surfacing them on the existing per-decision trace carrier (mirroring `opportunity_compiler_load`) keeps the instrumentation local, read-only, and recomputable — it never becomes authoritative state (FND-27). The aggregator reads it the same way it reads every other trace surface.
2. No backwards-compatibility aliasing/shims — the counter carrier is a net-new optional field; the `opportunity_compiler_load` precedent means no new pattern is introduced.

## Verification Layers

1. Cache-access counting accuracy (N `Some`/`None` results from `DistanceMatrix::get` → counter equals N) -> focused unit test in `planning_snapshot.rs`.
2. Counter carrier reaches the decision trace (a snapshot's counters appear on the produced `AgentDecisionTrace`) -> decision-trace assertion in a focused `decision_trace.rs` test.
3. No planning-behavior change (existing `min_travel_ticks_self_is_zero`, `min_travel_ticks_direct_adjacent`, `snapshot_filter_excludes_items_for_travel_only_goal` pass with identical values) -> existing focused unit tests named in Assumption Reassessment item 1.

## What to Change

### 1. Logical counters on `PlanningSnapshot` caches

Add read-only `u64` counters to `PlanningSnapshot` for cache hit/miss/invalidation. Instrument `DistanceMatrix::get` (or the `PlanningSnapshot` travel/cost accessors) so a `Some` result increments the hit counter and a `None` increments the miss counter. Per Assumption Reassessment item 4, `invalidation_count` is `0` for the precomputed `DistanceMatrix` unless a populate-on-demand cache is identified during reassessment — document the chosen mapping here before editing. Counters are read-only derived state — they never feed back into planning decisions.
`To be confirmed:` the precise instrumentation site (inside `DistanceMatrix::get` vs. the `PlanningSnapshot` public accessors) — pin during reassessment by reading `planning_snapshot.rs:316-410` and `:811-870`.

### 2. Counter carrier on `AgentDecisionTrace`

Add a new optional carrier field to `AgentDecisionTrace` (e.g. `snapshot_cache_counters: Option<SnapshotCacheCounters>`) holding the per-decision counter snapshot, following the `opportunity_compiler_load: Option<OpportunityCompilerLoad>` precedent. Define the `SnapshotCacheCounters` struct in `decision_trace.rs` with `Clone, Debug` derives (decision traces are not save/load state, so serde is not required).

### 3. Populate all `AgentDecisionTrace` construction sites

Add the new field to all 23 construction sites across the 7 files named in Assumption Reassessment item 3 — `None` for sites that produce no planning snapshot, the real counters for the agent-tick decision path.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — counters + access-path instrumentation)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — `SnapshotCacheCounters` struct + carrier field)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — populate carrier in the decision path)
- `crates/worldwake-ai/src/survival_forensics.rs` (modify — `AgentDecisionTrace` construction site)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — `AgentDecisionTrace` construction site)
- `crates/worldwake-visualizer/src/trace_buffers.rs` (modify — `AgentDecisionTrace` construction site)
- `crates/worldwake-ai/tests/golden_harness/timeline.rs` (modify — `AgentDecisionTrace` construction site)
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` (modify — `AgentDecisionTrace` construction site)

## Out of Scope

- Reading the counters into `ScenarioDiagnosticsReport` — that is the aggregator's job (ticket 005).
- Wall-clock timing of any kind — S144 forbids wall-clock data in the report.
- `OpportunityCompilerLoad` — already carried; the aggregator reads it directly with no instrumentation.
- Queue-wait metrics — derived by the aggregator from event-log tags, no instrumentation needed.

## Acceptance Criteria

### Tests That Must Pass

1. After N `DistanceMatrix::get` calls returning `Some`, the hit counter equals N; after M returning `None`, the miss counter equals M.
2. A `PlanningSnapshot`'s counters appear on the `AgentDecisionTrace` produced for that decision.
3. Existing `planning_snapshot.rs` tests `min_travel_ticks_self_is_zero`, `min_travel_ticks_direct_adjacent`, `snapshot_filter_excludes_items_for_travel_only_goal` pass with unchanged values.
4. Existing suite: `cargo test -p worldwake-ai` and `cargo test -p worldwake-visualizer`

### Invariants

1. Counters are read-only derived state — they never influence planning decisions (FND-27).
2. No planning-behavior change — every `PlanningSnapshot` accessor returns the same values it did before instrumentation.
3. `AgentDecisionTrace` remains free of save/load serialization requirements — the new carrier derives only `Clone, Debug`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` (inline `#[cfg(test)]`) — counter accuracy over known hit/miss sequences.
2. `crates/worldwake-ai/src/decision_trace.rs` (inline `#[cfg(test)]`) — `SnapshotCacheCounters` carrier reaches `AgentDecisionTrace`; extends the trace-construction coverage near the `dead_trace`/`goal_trace` helpers.

### Commands

1. `cargo test -p worldwake-ai planning_snapshot` and `cargo test -p worldwake-ai decision_trace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `./scripts/verify.sh` (cross-crate construction-site changes touch `worldwake-cli` and `worldwake-visualizer` — the full gate is the correct verification boundary)
