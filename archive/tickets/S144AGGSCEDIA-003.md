# S144AGGSCEDIA-003: PlanningSnapshot cache counters and decision-trace surfacing

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes -- read-only logical counters on `PlanningSnapshot` cache accesses; new carrier field on `AgentDecisionTrace`. No planning-behavior change.
**Deps**: None -- foundation ticket (S144 D8)

## Problem

S144's `PerformanceMetrics` needs deterministic logical cache hit/miss/invalidation counts. `PlanningSnapshot` holds precomputed travel/cost caches but counts no logical access events, and the decision trace carries no surface for them. Without this instrumentation, the aggregator (ticket 005) cannot populate `PerformanceMetrics.cache_*` fields.

## Assumption Reassessment (2026-05-14)

1. `PlanningSnapshot` owns two precomputed `DistanceMatrix` fields: `shortest_travel_ticks` and `perceived_travel_costs`. The matrix-backed accessors are `min_travel_ticks`, `min_travel_ticks_to_any`, and `min_perceived_travel_cost_to_any`; `direct_perceived_travel_cost` / `direct_perceived_travel_breakdown` compute a direct-edge breakdown from place adjacency and route-threat state, so they are not counted as matrix-cache lookups. Existing inline tests exercising this surface (`min_travel_ticks_self_is_zero`, `min_travel_ticks_direct_adjacent`, `snapshot_filter_excludes_items_for_travel_only_goal`) must continue to pass; the instrumentation is purely additive and changes no return values.
2. S144 spec D8 (`specs/S144-aggregate-scenario-diagnostics.md`) specifies read-only `u64` logical counters on the snapshot caches surfaced through the decision trace, explicitly framed as a derived read-model addition (FND-27) with no planning-behavior change. `AgentDecisionTrace` (`decision_trace.rs:94`, derives `Clone, Debug`) already carries an analogous optional load carrier `opportunity_compiler_load: Option<OpportunityCompilerLoad>` (:99) -- the new counter carrier follows that precedent.
3. Mixed-layer data contract under audit: `AgentDecisionTrace` gains a new carrier field. `AgentDecisionTrace` has 23 construction sites across 7 files (`agent_tick/mod.rs`, `decision_trace.rs`, `survival_forensics.rs`, `bin/observer.rs`, `worldwake-visualizer/src/trace_buffers.rs`, `tests/golden_harness/timeline.rs`, `tests/golden_harness/survival_forensics_assertions.rs`); `AgentDecisionTrace` derives no `Default`, so every site must add the new field explicitly (as `None` for sites that produce no planning snapshot).
4. Adjacent-contradiction classification (required consequence of D8, not a separate bug): `DistanceMatrix` is a *precomputed* matrix, not a populate-on-demand cache. The implemented mapping is: a matrix-backed accessor whose underlying lookup returns `Some(_)` increments `cache_hit_count`; `None` increments `cache_miss_count`; `cache_invalidation_count` is always `0` because the matrices are rebuilt with the snapshot rather than invalidated incrementally.

## Architecture Check

1. Counting logical cache accesses inside `PlanningSnapshot` and surfacing them on the existing per-decision trace carrier (mirroring `opportunity_compiler_load`) keeps the instrumentation local, read-only, and recomputable -- it never becomes authoritative state (FND-27). The aggregator reads it the same way it reads every other trace surface.
2. No backwards-compatibility aliasing/shims -- the counter carrier is a net-new optional field; the `opportunity_compiler_load` precedent means no new pattern is introduced.

## Verified Layers

1. Cache-access counting accuracy (N `Some`/`None` results from `DistanceMatrix::get` -> counter equals N) -> focused unit test in `planning_snapshot.rs`.
2. Counter carrier reaches the decision trace (a snapshot's counters appear on the produced `AgentDecisionTrace`) -> decision-trace assertion in a focused `decision_trace.rs` test.
3. No planning-behavior change (existing `min_travel_ticks_self_is_zero`, `min_travel_ticks_direct_adjacent`, `snapshot_filter_excludes_items_for_travel_only_goal` pass with identical values) -> existing focused unit tests named in Assumption Reassessment item 1.

## Landed Changes

### 1. Logical counters on `PlanningSnapshot` caches

Added read-only `u64` counters to `PlanningSnapshot` for matrix-backed cache hit/miss/invalidation. The instrumentation lives in the `PlanningSnapshot` travel/cost accessors, not inside `DistanceMatrix::get`, so self fast-paths and direct-edge breakdown helpers are not misreported as matrix lookups. Counters are read-only derived state and never feed back into planning decisions.

### 2. Counter carrier on `AgentDecisionTrace`

Added `AgentDecisionTrace.snapshot_cache_counters: Option<SnapshotCacheCounters>` holding the per-decision counter snapshot, following the `opportunity_compiler_load: Option<OpportunityCompilerLoad>` precedent. `SnapshotCacheCounters` lives in `decision_trace.rs` and derives `Clone, Copy, Debug, Default, Eq, PartialEq`; decision traces are not save/load state, so serde was not added.

### 3. Populate all `AgentDecisionTrace` construction sites

Added the new field to all live `AgentDecisionTrace` construction sites. Sites that produce no planning snapshot use `None`; the traced agent-tick planning path records the aggregate counters from planning snapshots built during candidate-plan search.

## Landed Files

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify -- counters + access-path instrumentation)
- `crates/worldwake-ai/src/decision_trace.rs` (modify -- `SnapshotCacheCounters` struct + carrier field)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify -- populate carrier in the decision path)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify -- aggregate counters from candidate-plan snapshots)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify -- tuple fallout from trace result widening)
- `crates/worldwake-ai/src/survival_forensics.rs` (modify -- `AgentDecisionTrace` construction site)
- `crates/worldwake-cli/src/bin/observer.rs` (modify -- `AgentDecisionTrace` construction site)
- `crates/worldwake-visualizer/src/trace_buffers.rs` (modify -- `AgentDecisionTrace` construction site)
- `crates/worldwake-ai/tests/golden_harness/timeline.rs` (modify -- `AgentDecisionTrace` construction site)
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` (modify -- `AgentDecisionTrace` construction site)
- `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` (modify -- real agent-tick trace carrier assertion)
- `crates/worldwake-ai/src/lib.rs` (modify -- public re-export of `SnapshotCacheCounters`)
- `specs/S144-aggregate-scenario-diagnostics.md` (modify -- D8 mapping truth-sync)

## Out of Scope

- Reading the counters into `ScenarioDiagnosticsReport` -- that is the aggregator's job (ticket 005).
- Wall-clock timing of any kind -- S144 forbids wall-clock data in the report.
- `OpportunityCompilerLoad` -- already carried; the aggregator reads it directly with no instrumentation.
- Queue-wait metrics -- derived by the aggregator from event-log tags, no instrumentation needed.

## Acceptance Result

### Tests Passed

1. Passed: after matrix-backed accessors produce `Some` / `None`, `SnapshotCacheCounters` reports matching hit/miss totals and zero invalidations.
2. Passed: a real agent-tick decision trace carries `snapshot_cache_counters` on the planning path.
3. Passed: existing `planning_snapshot.rs` tests `min_travel_ticks_self_is_zero`, `min_travel_ticks_direct_adjacent`, `snapshot_filter_excludes_items_for_travel_only_goal` pass with unchanged return values.
4. Passed: `cargo test -p worldwake-ai` and `cargo test -p worldwake-visualizer`.
5. Passed: `cargo clippy --workspace --all-targets -- -D warnings` and `./scripts/verify.sh`.

### Invariants

1. Counters are read-only derived state -- they never influence planning decisions (FND-27).
2. No planning-behavior change -- every `PlanningSnapshot` accessor returns the same values it did before instrumentation.
3. `AgentDecisionTrace` remains free of save/load serialization requirements -- the new carrier derives only `Clone, Debug`.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` (inline `#[cfg(test)]`) -- counter accuracy over known hit/miss sequences.
2. `crates/worldwake-ai/src/decision_trace.rs` (inline `#[cfg(test)]`) -- `SnapshotCacheCounters` carrier reaches `DecisionTraceSink` by agent/tick.
3. `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` -- existing real agent-tick trace test now asserts `snapshot_cache_counters` is surfaced on the produced trace.

### Verification Commands

1. Passed `cargo test --workspace --no-run`
2. Passed `cargo test -p worldwake-ai --lib planning_snapshot::tests::snapshot_cache_counters_record_matrix_hits_and_misses -- --exact`
3. Passed `cargo test -p worldwake-ai --lib decision_trace::tests::sink_records_snapshot_cache_counters_by_agent_tick -- --exact`
4. Passed `cargo test -p worldwake-ai --test golden_opportunity_compiler agent_tick_trace_carries_compiled_opportunities_and_load -- --exact`
5. Passed `cargo test -p worldwake-ai --lib planning_snapshot::tests::min_travel_ticks_self_is_zero -- --exact`
6. Passed `cargo test -p worldwake-ai --lib planning_snapshot::tests::min_travel_ticks_direct_adjacent -- --exact`
7. Passed `cargo test -p worldwake-ai --lib planning_snapshot::tests::snapshot_filter_excludes_items_for_travel_only_goal -- --exact`
8. Passed `cargo test -p worldwake-ai`
9. Passed `cargo test -p worldwake-visualizer`
10. Passed `cargo clippy --workspace --all-targets -- -D warnings`
11. Passed `./scripts/verify.sh`

## Outcome

Completed implementation and verification on 2026-05-14.

- Added deterministic logical cache counters to the `PlanningSnapshot` precomputed matrix accessors.
- Surfaced aggregate per-decision counters through `AgentDecisionTrace.snapshot_cache_counters` and `DecisionTraceSink`.
- Threaded real counters from traced candidate-plan search into the produced agent-tick decision trace.
- Updated all live `AgentDecisionTrace` construction sites with explicit `None` where no planning snapshot exists.
- Truth-synced S144 D8 to the implemented precomputed-matrix mapping.

## Deviations

- The counter instrumentation is deliberately on `PlanningSnapshot` accessors rather than inside `DistanceMatrix::get`, so direct-edge route breakdown helpers are not counted as matrix-cache accesses.
- `cache_invalidation_count` remains `0` because the live matrices are rebuilt with each snapshot and have no incremental invalidation lifecycle.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-ai --lib planning_snapshot::tests::snapshot_cache_counters_record_matrix_hits_and_misses -- --exact`
- Passed `cargo test -p worldwake-ai --lib decision_trace::tests::sink_records_snapshot_cache_counters_by_agent_tick -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_opportunity_compiler agent_tick_trace_carries_compiled_opportunities_and_load -- --exact`
- Passed `cargo test -p worldwake-ai --lib planning_snapshot::tests::min_travel_ticks_self_is_zero -- --exact`
- Passed `cargo test -p worldwake-ai --lib planning_snapshot::tests::min_travel_ticks_direct_adjacent -- --exact`
- Passed `cargo test -p worldwake-ai --lib planning_snapshot::tests::snapshot_filter_excludes_items_for_travel_only_goal -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-visualizer`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `./scripts/verify.sh`
