# S145PLASUBHAR-003: Planning-state cache counters

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `PlanningStateCacheCounters` type, optional field on `AgentDecisionTrace`, `PlanningState` read-path instrumentation, `PerformanceMetrics` extension, S144 fixture regeneration
**Deps**: None

## Problem

S144 shipped `SnapshotCacheCounters` at `crates/worldwake-ai/src/decision_trace.rs:107` for `PlanningSnapshot`'s precomputed `DistanceMatrix` caches and surfaces them through `AgentDecisionTrace.snapshot_cache_counters` (`decision_trace.rs:100`) aggregated into `PerformanceMetrics.cache_hit_count` / `cache_miss_count` / `cache_invalidation_count`. `PlanningState`'s `entities_at_cache` and `effective_place_cache` (`crates/worldwake-ai/src/planning_state.rs:71-72`) are a separate cache substrate that S144 did not cover — they have no per-run counter visibility today. Per S145 D4, a parallel runtime counter type with per-cache breakdown (`entities_at_hits`, `entities_at_misses`, `effective_place_hits`, `effective_place_misses`, `invalidations`) must surface through `AgentDecisionTrace` and extend `PerformanceMetrics` additively without repurposing the existing flat fields.

## Assumption Reassessment (2026-05-16)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `SnapshotCacheCounters` exists at `crates/worldwake-ai/src/decision_trace.rs:107` deriving `Clone, Copy, Debug, Default, Eq, PartialEq` with `is_empty()` and `add_assign()` methods (lines 113-128). The new `PlanningStateCacheCounters` follows this exact shape (S145 reassessment finding M3) so the per-tick / per-run aggregation pattern carries over without invention. `AgentDecisionTrace.snapshot_cache_counters: Option<SnapshotCacheCounters>` at `:100` is the precedent for the new field's placement; the run-level aggregator at `decision_trace.rs:1418` (`snapshot_cache_counters: BTreeMap<(EntityId, Tick), SnapshotCacheCounters>`) is the precedent for cross-tick aggregation.
2. Seven `AgentDecisionTrace` construction sites exist workspace-wide and none use spread syntax: `crates/worldwake-cli/src/bin/observer.rs:6140`, `crates/worldwake-visualizer/src/trace_buffers.rs:249`, `:263` (both inside `#[cfg(test)]` block starting at `:155`), `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs:202`, `crates/worldwake-ai/src/survival_forensics.rs:736`, `crates/worldwake-ai/src/decision_trace.rs:2823`, `:2887`. All seven sites add `planning_state_cache_counters: None`.
3. Shared abstraction boundary: `PlanningState` (`crates/worldwake-ai/src/planning_state.rs`) is the planner's mutable substrate for hypothetical branch evaluation; the caches at `:71-72` are pure-function memoization invalidated by six mutators (`move_lot_ref_to_holder:407`, `move_lot_ref_to_ground:434`, `move_entity_ref:571`, `set_possessor_ref:583`, `set_container_ref:596`, `mark_removed_ref:618`). Counter instrumentation lands at the read paths (`entities_at` at `:1293-1338`, `effective_place_ref` at `:486-491`, internal `effective_place` cache lookup at `:911-930`) and at `invalidate_entities_at_cache:106-109`. No new boundary introduced — counters are pure observability per FND-27.
4. Adjacent contradiction surfaced during reassessment: spec S145's Crates section lists only 5 workspace crates (core, sim, systems, ai, cli) but `crates/worldwake-visualizer/` exists and constructs `AgentDecisionTrace` at two `#[cfg(test)]` sites. The visualizer must be touched even though the spec didn't anticipate it. Classification: separate adjacent finding — surface in this ticket's Files to Touch; no follow-up ticket needed because the visualizer change is a trivial field addition.

## Architecture Check

1. The per-cache breakdown (5 fields instead of S144's 3 flat fields) is intentional: `entities_at` and `effective_place` answer different planner queries and may have different hit ratios — collapsing them would lose diagnostic signal. The `SnapshotCacheCounters` shape (3 flat fields) is correct for the matrix-cache surface where the cache type is uniform, but `PlanningState` caches are heterogeneous in query semantics.
2. The `PerformanceMetrics` extension is strictly additive: the existing `cache_hit_count` / `cache_miss_count` / `cache_invalidation_count` fields continue to report only `SnapshotCacheCounters`; new `planning_state_cache_*` fields report only `PlanningStateCacheCounters`. Per FND-28, no dual-truth — each field has exactly one authoritative source. S144's already-archived spec is not retroactively modified; the additive extension is documented as an S145 deliverable in `specs/S145-planning-substrate-hardening.md` D4.

## Verification Layers

1. Counter increments correctly across the full mutation surface (6 invalidating + 1 no-op mutator) → focused unit test owned by ticket S145PLASUBHAR-004 (`cache_invalidation_count_increments_on_each_mutation`); this ticket's smoke test confirms initial-state-zero only.
2. `AgentDecisionTrace.planning_state_cache_counters` field is structurally present → existing golden tests that construct `AgentDecisionTrace` and serialize PerformanceMetrics fixtures will fail at compile-time if the field is missing.
3. `PerformanceMetrics` aggregator surfaces new fields → the regenerated `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` is byte-stable on the survival-baseline scenario at the project seed (deterministic per S144 D10).
4. Single observability ticket spanning decision-trace and aggregator layers; FND-29 debuggability is the contract. No action-trace or event-log surface — counters do not represent world events.

## What to Change

### 1. Define `PlanningStateCacheCounters` in `decision_trace.rs`

In `crates/worldwake-ai/src/decision_trace.rs`, add a new type alongside `SnapshotCacheCounters`:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlanningStateCacheCounters {
    pub entities_at_hits: u64,
    pub entities_at_misses: u64,
    pub effective_place_hits: u64,
    pub effective_place_misses: u64,
    pub invalidations: u64,
}

impl PlanningStateCacheCounters {
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.entities_at_hits == 0
            && self.entities_at_misses == 0
            && self.effective_place_hits == 0
            && self.effective_place_misses == 0
            && self.invalidations == 0
    }

    pub fn add_assign(&mut self, other: Self) {
        self.entities_at_hits = self.entities_at_hits.saturating_add(other.entities_at_hits);
        self.entities_at_misses = self.entities_at_misses.saturating_add(other.entities_at_misses);
        self.effective_place_hits = self.effective_place_hits.saturating_add(other.effective_place_hits);
        self.effective_place_misses = self.effective_place_misses.saturating_add(other.effective_place_misses);
        self.invalidations = self.invalidations.saturating_add(other.invalidations);
    }
}
```

Re-export from `crates/worldwake-ai/src/lib.rs` alongside the existing `SnapshotCacheCounters` re-export at line 89.

### 2. Add optional field to `AgentDecisionTrace`

In the same file (struct at `:94`), add:

```rust
pub planning_state_cache_counters: Option<PlanningStateCacheCounters>,
```

Mirror the existing `snapshot_cache_counters: Option<SnapshotCacheCounters>` field placement (`:100`).

If the run-level aggregator at `:1418` rolls up `snapshot_cache_counters`, extend it with a parallel `planning_state_cache_counters: BTreeMap<(EntityId, Tick), PlanningStateCacheCounters>` field and matching `add_assign` accumulation at `:1435`.

### 3. Instrument `PlanningState` read paths and invalidation

In `crates/worldwake-ai/src/planning_state.rs`, add a `counters: Cell<PlanningStateCacheCounters>` (or equivalent shared-interior-mutability) field to `PlanningState` so the read paths can bump counters without an exclusive borrow. Zero-initialize in `PlanningState::new` (line 78).

At the read sites, bump the appropriate hit/miss counter:

- `entities_at` at `:1293-1338`: bump `entities_at_hits` on cache hit path (`:1312-1314`); bump `entities_at_misses` on slow-path that inserts into cache (`:1316-1337`).
- `effective_place_ref` at `:486-491`: bump `effective_place_hits` on cache hit, `effective_place_misses` on insert path.
- Internal `effective_place` cache read at `:911-930`: same treatment.

In `invalidate_entities_at_cache` at `:106-109`: bump `invalidations` by 1 before resetting the caches.

Expose the counters via a `pub fn cache_counters(&self) -> PlanningStateCacheCounters` accessor so callers can snapshot the counter state when constructing `AgentDecisionTrace`.

### 4. Update all seven `AgentDecisionTrace` construction sites

Add `planning_state_cache_counters: None` (or populated `Some(counters)` at the call sites that own a live `PlanningState`) at each enumerated site:

- `crates/worldwake-cli/src/bin/observer.rs:6140`
- `crates/worldwake-visualizer/src/trace_buffers.rs:249`
- `crates/worldwake-visualizer/src/trace_buffers.rs:263`
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs:202`
- `crates/worldwake-ai/src/survival_forensics.rs:736`
- `crates/worldwake-ai/src/decision_trace.rs:2823`
- `crates/worldwake-ai/src/decision_trace.rs:2887`

At least one runtime call site (typically `agent_tick`-driven `AgentDecisionTrace` construction) should populate `Some(state.cache_counters())` so the new fields carry non-zero values into S144's aggregator on real runs.

### 5. Extend `PerformanceMetrics` in the aggregator

In `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (or wherever `PerformanceMetrics` is defined per S144 D1), add five new fields alongside the existing `cache_hit_count` / `cache_miss_count` / `cache_invalidation_count`:

```rust
pub planning_state_cache_entities_at_hits: u64,
pub planning_state_cache_entities_at_misses: u64,
pub planning_state_cache_effective_place_hits: u64,
pub planning_state_cache_effective_place_misses: u64,
pub planning_state_cache_invalidations: u64,
```

Extend the `build_scenario_diagnostics` aggregator to walk `AgentDecisionTrace.planning_state_cache_counters` and sum into these fields in the same single pass it uses for `snapshot_cache_counters`.

### 6. Regenerate the S144 diagnostics fixture

Regenerate `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` for `scenarios/survival-baseline.ron` at the project's standard seed. Verify byte stability by running the regen twice and confirming identical output (S144 D10's determinism contract carries over). Commit the regenerated fixture in the same diff.

### 7. Render `PlanningStateCacheCounters` in observer

Mirror the existing `SnapshotCacheCounters` observer rendering (whichever section it appears in) so the new counters surface in the operator-facing diagnostics output. The render is additive and follows the existing format.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — new type, new field on `AgentDecisionTrace`, run-level aggregator extension at `:1418`+, two construction sites at `:2823, :2887`)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export `PlanningStateCacheCounters`)
- `crates/worldwake-ai/src/planning_state.rs` (modify — `counters` field, read-path instrumentation at `:486-491, :911-930, :1293-1338`, invalidation bump at `:106-109`, `cache_counters()` accessor)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — `PerformanceMetrics` extension, aggregator walks the new field)
- `crates/worldwake-ai/src/survival_forensics.rs` (modify — construction site at `:736`)
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` (modify — construction site at `:202`)
- `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (modify — regenerated)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — construction site at `:6140`, optional render for the new counters)
- `crates/worldwake-visualizer/src/trace_buffers.rs` (modify — construction sites at `:249, :263`; spec-gap crate not listed in S145's Crates section)

## Out of Scope

- No change to existing `SnapshotCacheCounters` or its three flat `PerformanceMetrics` fields — they continue to report only the `PlanningSnapshot` matrix cache surface (FND-28 single-truth).
- No replacement of `Rc<RefCell<...>>` with a different concurrency primitive — per S145 Non-Goals, this is rejected as premature optimization.
- No incremental snapshot mechanism — that addresses PR-9, not S145.
- Counter-increment correctness tests across the full mutation surface — that is S145PLASUBHAR-004 D3.5.
- Module-level cache-invariant doc comment — that is S145PLASUBHAR-004 D5 (subsumed).

## Acceptance Criteria

### Tests That Must Pass

1. Smoke test in `crates/worldwake-ai/src/planning_state.rs` `#[cfg(test)]` module: a freshly-constructed `PlanningState::new` returns `cache_counters().is_empty() == true`.
2. The regenerated `expected-scenario-diagnostics.json` matches the new shape; existing `crates/worldwake-ai/tests/golden_scenario_diagnostics.rs` determinism golden continues to pass.
3. Existing `cargo test -p worldwake-ai` and `cargo test -p worldwake-visualizer` pass with the structural field additions.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. `PerformanceMetrics` carries both flat S144 cache counter fields AND the five new `planning_state_cache_*` fields — neither shape replaces the other (FND-28).
2. `invalidations` counter increments by exactly 1 per call to `invalidate_entities_at_cache` — verified in ticket 004's D3.5 test.
3. `PlanningStateCacheCounters::add_assign` is associative and commutative — the run-level aggregator (`BTreeMap<(EntityId, Tick), _>`) walked in any order produces identical totals.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_state.rs` (modify, `#[cfg(test)]` module) — initial-zero smoke test only; counter-increment correctness is owned by ticket 004 (D3.5).
2. `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (regenerate) — byte-stable on `survival-baseline.ron` at project seed.

### Commands

1. `cargo test -p worldwake-ai planning_state`
2. `cargo test -p worldwake-ai --test golden_scenario_diagnostics`
3. `cargo test --workspace`
4. `scripts/verify.sh`
