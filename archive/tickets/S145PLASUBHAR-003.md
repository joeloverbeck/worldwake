# S145PLASUBHAR-003: Planning-state cache counters

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `PlanningStateCacheCounters` type, optional field on `AgentDecisionTrace`, `PlanningState` read-path instrumentation, `PerformanceMetrics` extension, S144 fixture regeneration
**Deps**: None

## Problem

Before this ticket, S144 had shipped `SnapshotCacheCounters` at `crates/worldwake-ai/src/decision_trace.rs:107` for `PlanningSnapshot`'s precomputed `DistanceMatrix` caches and surfaced them through `AgentDecisionTrace.snapshot_cache_counters` (`decision_trace.rs:100`) aggregated into `PerformanceMetrics.cache_hit_count` / `cache_miss_count` / `cache_invalidation_count`. `PlanningState`'s `entities_at_cache` and `effective_place_cache` (`crates/worldwake-ai/src/planning_state.rs:71-72`) were a separate cache substrate that S144 did not cover and had no per-run counter visibility. S145 D4 required a parallel runtime counter type with per-cache breakdown (`entities_at_hits`, `entities_at_misses`, `effective_place_hits`, `effective_place_misses`, `invalidations`) surfaced through `AgentDecisionTrace` and additive `PerformanceMetrics` fields without repurposing the existing flat fields.

## Assumption Reassessment (2026-05-16)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `SnapshotCacheCounters` exists at `crates/worldwake-ai/src/decision_trace.rs:107` deriving `Clone, Copy, Debug, Default, Eq, PartialEq` with `is_empty()` and `add_assign()` methods (lines 113-128). The new `PlanningStateCacheCounters` follows this exact shape (S145 reassessment finding M3) so the per-tick / per-run aggregation pattern carries over without invention. `AgentDecisionTrace.snapshot_cache_counters: Option<SnapshotCacheCounters>` at `:100` is the precedent for the new field's placement; the run-level aggregator at `decision_trace.rs:1418` (`snapshot_cache_counters: BTreeMap<(EntityId, Tick), SnapshotCacheCounters>`) is the precedent for cross-tick aggregation.
2. The final constructor sweep found the drafted seven `AgentDecisionTrace` construction sites plus additional live runtime/test constructors in `agent_tick`, `decision_trace`, `scenario_diagnostics`, and golden harness helpers. All direct constructors now set `planning_state_cache_counters`; the `agent_tick` planning path populates `Some(_)` from search metadata, while non-planning/dead/test helpers use `None`.
3. Shared abstraction boundary: `PlanningState` (`crates/worldwake-ai/src/planning_state.rs`) is the planner's mutable substrate for hypothetical branch evaluation; the caches at `:71-72` are pure-function memoization invalidated by six mutators (`move_lot_ref_to_holder:407`, `move_lot_ref_to_ground:434`, `move_entity_ref:571`, `set_possessor_ref:583`, `set_container_ref:596`, `mark_removed_ref:618`). Counter instrumentation lands at the read paths (`entities_at` at `:1293-1338`, `effective_place_ref` at `:486-491`, internal `effective_place` cache lookup at `:911-930`) and at `invalidate_entities_at_cache:106-109`. No new boundary introduced — counters are pure observability per FND-27.
4. Adjacent contradiction surfaced during reassessment: spec S145's Crates section lists only 5 workspace crates (core, sim, systems, ai, cli) but `crates/worldwake-visualizer/` exists and constructs `AgentDecisionTrace` at two `#[cfg(test)]` sites. The visualizer must be touched even though the spec didn't anticipate it. Classification: separate adjacent finding — surface in this ticket's Files to Touch; no follow-up ticket needed because the visualizer change is a trivial field addition.

## Architecture Check

1. The per-cache breakdown (5 fields instead of S144's 3 flat fields) is intentional: `entities_at` and `effective_place` answer different planner queries and may have different hit ratios — collapsing them would lose diagnostic signal. The `SnapshotCacheCounters` shape (3 flat fields) is correct for the matrix-cache surface where the cache type is uniform, but `PlanningState` caches are heterogeneous in query semantics.
2. The `PerformanceMetrics` extension is strictly additive: the existing `cache_hit_count` / `cache_miss_count` / `cache_invalidation_count` fields continue to report only `SnapshotCacheCounters`; new `planning_state_cache_*` fields report only `PlanningStateCacheCounters`. Per FND-28, no dual-truth — each field has exactly one authoritative source. S144's already-archived spec is not retroactively modified; the additive extension is documented as an S145 deliverable in `specs/S145-planning-substrate-hardening.md` D4.

## Verified Layers

1. Counter increments correctly across the full mutation surface (6 invalidating + 1 no-op mutator) → focused unit test owned by ticket S145PLASUBHAR-004 (`cache_invalidation_count_increments_on_each_mutation`); this ticket's smoke test confirms initial-state-zero only.
2. `AgentDecisionTrace.planning_state_cache_counters` field is structurally present → existing golden tests that construct `AgentDecisionTrace` and serialize PerformanceMetrics fixtures will fail at compile-time if the field is missing.
3. `PerformanceMetrics` aggregator surfaces new fields → the regenerated `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` is byte-stable on the survival-baseline scenario at the project seed (deterministic per S144 D10).
4. Single observability ticket spanning decision-trace and aggregator layers; FND-29 debuggability is the contract. No action-trace or event-log surface — counters do not represent world events.

## Landed Changes

### 1. Defined `PlanningStateCacheCounters` in `decision_trace.rs`

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

### 2. Added optional field to `AgentDecisionTrace`

In the same file (struct at `:94`), add:

```rust
pub planning_state_cache_counters: Option<PlanningStateCacheCounters>,
```

Mirror the existing `snapshot_cache_counters: Option<SnapshotCacheCounters>` field placement (`:100`).

If the run-level aggregator at `:1418` rolls up `snapshot_cache_counters`, extend it with a parallel `planning_state_cache_counters: BTreeMap<(EntityId, Tick), PlanningStateCacheCounters>` field and matching `add_assign` accumulation at `:1435`.

### 3. Instrumented `PlanningState` read paths and invalidation

In `crates/worldwake-ai/src/planning_state.rs`, add a `counters: Cell<PlanningStateCacheCounters>` (or equivalent shared-interior-mutability) field to `PlanningState` so the read paths can bump counters without an exclusive borrow. Zero-initialize in `PlanningState::new` (line 78).

At the read sites, bump the appropriate hit/miss counter:

- `entities_at` at `:1293-1338`: bump `entities_at_hits` on cache hit path (`:1312-1314`); bump `entities_at_misses` on slow-path that inserts into cache (`:1316-1337`).
- `effective_place_ref` at `:486-491`: bump `effective_place_hits` on cache hit, `effective_place_misses` on insert path.
- Internal `effective_place` cache read at `:911-930`: same treatment.

In `invalidate_entities_at_cache` at `:106-109`: bump `invalidations` by 1 before resetting the caches.

Expose the counters via a `pub fn cache_counters(&self) -> PlanningStateCacheCounters` accessor so callers can snapshot the counter state when constructing `AgentDecisionTrace`.

### 4. Updated all `AgentDecisionTrace` construction sites

All direct `AgentDecisionTrace` construction sites were updated. Runtime planning traces now carry `Some(planning_state_cache_counters)` from the search path; non-planning/dead/test constructors use `None`.

### 5. Extended `PerformanceMetrics` in the aggregator

In `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (or wherever `PerformanceMetrics` is defined per S144 D1), add five new fields alongside the existing `cache_hit_count` / `cache_miss_count` / `cache_invalidation_count`:

```rust
pub planning_state_cache_entities_at_hits: u64,
pub planning_state_cache_entities_at_misses: u64,
pub planning_state_cache_effective_place_hits: u64,
pub planning_state_cache_effective_place_misses: u64,
pub planning_state_cache_invalidations: u64,
```

Extend the `build_scenario_diagnostics` aggregator to walk `AgentDecisionTrace.planning_state_cache_counters` and sum into these fields in the same single pass it uses for `snapshot_cache_counters`.

### 6. Regenerated the S144 diagnostics fixture

Regenerate `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` for `scenarios/survival-baseline.ron` at the project's standard seed. Verify byte stability by running the regen twice and confirming identical output (S144 D10's determinism contract carries over). Commit the regenerated fixture in the same diff.

### 7. Rendered `PlanningStateCacheCounters` in observer

Mirror the existing `SnapshotCacheCounters` observer rendering (whichever section it appears in) so the new counters surface in the operator-facing diagnostics output. The render is additive and follows the existing format.

## Landed Files

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

## Acceptance Result

### Tests That Passed

1. Smoke test in `crates/worldwake-ai/src/planning_state.rs` `#[cfg(test)]` module: a freshly-constructed `PlanningState::new` returns `cache_counters().is_empty() == true`.
2. The regenerated `expected-scenario-diagnostics.json` matches the new shape; the ignored fixture stability and replay determinism checks both pass.
3. Existing `cargo test -p worldwake-ai` and `cargo test -p worldwake-visualizer` pass with the structural field additions.
4. Workspace wrapper verification was not run; see the waiver in `Commands Run`.

### Invariants

1. `PerformanceMetrics` carries both flat S144 cache counter fields AND the five new `planning_state_cache_*` fields — neither shape replaces the other (FND-28).
2. `invalidations` counter increments by exactly 1 per call to `invalidate_entities_at_cache` — verified in ticket 004's D3.5 test.
3. `PlanningStateCacheCounters::add_assign` is associative and commutative — the run-level aggregator (`BTreeMap<(EntityId, Tick), _>`) walked in any order produces identical totals.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/planning_state.rs` (modify, `#[cfg(test)]` module) — initial-zero smoke test only; counter-increment correctness is owned by ticket 004 (D3.5).
2. `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (regenerate) — byte-stable on `survival-baseline.ron` at project seed.

### Commands Run

1. Passed `cargo test -p worldwake-ai planning_state`
2. Passed `cargo test -p worldwake-ai --test golden_scenario_diagnostics_fixture`
3. Passed `cargo test -p worldwake-ai --test golden_scenario_diagnostics_fixture -- --ignored`
4. Passed `cargo test -p worldwake-ai --test golden_scenario_diagnostics_replay -- --ignored`
5. Passed `cargo test -p worldwake-visualizer`
6. Passed `cargo test -p worldwake-ai`
7. Waived `cargo test --workspace` and `scripts/verify.sh` because the ticket-owned surface is contained in `worldwake-ai`, `worldwake-cli`, and `worldwake-visualizer`; `cargo test -p worldwake-ai` plus the visualizer package and ignored diagnostics fixture/replay covered the changed trace, diagnostics, fixture, and constructor surfaces.

## Outcome

Completed on 2026-05-16.

- Added `PlanningStateCacheCounters`, `AgentDecisionTrace.planning_state_cache_counters`, `DecisionTraceSink` storage/query helpers, and a shared-counter implementation on `PlanningState`.
- Wired the search/agent-tick path so real planning decisions aggregate `PlanningState` cache counters into `AgentDecisionTrace`, then into `PerformanceMetrics`.
- Extended observer text/JSON diagnostics and regenerated `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` with the new `planning_state_cache_*` fields.
- Updated every direct `AgentDecisionTrace` constructor found by the live compiler/sweep, including additional runtime and test constructors beyond the original seven-site reassessment.

## Deviations

- `PlanningState` stores counters as `Rc<Cell<PlanningStateCacheCounters>>` rather than a plain `Cell` so branch clones share one aggregate diagnostic counter lineage with the existing shared cache substrate.
- The final constructor surface was wider than the original ticket enumeration; the extra sites were mechanical trace-shape updates, not a new architectural boundary.

## Verification Result

- Passed `cargo test -p worldwake-ai planning_state::tests::planning_state_cache_counters_start_empty`
- Passed `cargo test -p worldwake-ai --test golden_scenario_diagnostics_fixture`
- Passed `cargo test -p worldwake-ai --test golden_scenario_diagnostics_fixture -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_scenario_diagnostics_replay -- --ignored`
- Passed `cargo test -p worldwake-ai planning_state`
- Passed `cargo test -p worldwake-visualizer`
- Passed `cargo test -p worldwake-ai`
