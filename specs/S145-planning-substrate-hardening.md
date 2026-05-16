# S145: Planning Substrate Hardening

**Status**: Draft

## Summary

The external assessment in `reports/ai-architecture-improvements.md` flags two narrow planner-substrate concerns that the rest of the assessment's larger architectural proposals depend on:

1. **Strategic search budget collapses on long acquisition chains** (PR-16): Before `S145PLASUBHAR-001` landed, the formula lived in the private helper `strategic_search_budget` at `crates/worldwake-ai/src/search/strategic.rs`, which returned `usize::max(1, usize::from(execution_budget.max_prerequisite_locations()) * 2)`. With the default `max_prerequisite_locations = 3` (set in `impl Default for ExecutionBudget`), every strategic search got 6 expansions regardless of how many stages the goal decomposed into. A 5-stage production chain got the same budget as a 1-stage hunger goal, which meant longer chains could exhaust the strategic budget at the first wave of permutations and never reach the deeper stage permutations. `S145PLASUBHAR-001` moved the formula to `ExecutionBudget::strategic_budget_for_stages`; `S145PLASUBHAR-002` added per-attempt `StrategicBudgetTrace` provenance while the typed-terminal work remains deferred to S149.

2. **Shared planning caches lack a compound-order regression** (PR-18): `crates/worldwake-ai/src/planning_state.rs:71-72` uses `Rc<RefCell<BTreeMap<...>>>` for `entities_at_cache` and `effective_place_cache`. The invalidation function `invalidate_entities_at_cache` at lines 106–109 is called from six mutators across the move/possession/container/removal axes (`move_lot_ref_to_holder:407`, `move_lot_ref_to_ground:434`, `move_entity_ref:571`, `set_possessor_ref:583`, `set_container_ref:596`, `mark_removed_ref:618`). Existing tests at `planning_state.rs:4179` and `:4210` already cover *single-mutation* cross-clone independence. What is missing is a *compound-order* invariant: two sibling search branches that apply the same set of mutations in opposite orders must produce equal cache results. For a deterministic simulation, accidental cache order dependence is silently corruptive.

S145 lands both fixes as small, narrowly-scoped architectural improvements. The strategic-budget fix multiplies by `stages.len()` so the budget scales with decomposition depth and deletes the now-dead private helper. The cache fix adds a compound-order regression test plus parallel cache counters (analogous to the already-shipped `SnapshotCacheCounters` for `PlanningSnapshot`) and a documented invariant in code comments so the cache shape is reviewable and the order-independence claim is enforceable.

This spec is intentionally narrow. It is a pre-requisite for S146 (Goal Schema and Per-Goal Budgets) — which will further refine these surfaces — but ships separately because (a) it has no dependency on S146's larger registry refactor and (b) it gives S146 a stable substrate to build on.

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — owns the strategic-budget formula change in `search/strategic.rs`, the planning-state cache counters and invariant tests in `planning_state.rs`, the `StrategicBudgetTrace` field on `PlanAttemptTrace` and the `PlanningStateCacheCounters` type in `decision_trace.rs`.
- `worldwake-core` — exposes a new `ExecutionBudget::strategic_budget_for_stages(stage_count: usize) -> usize` method so the formula is testable in core without reaching into AI-crate internals.
- `worldwake-cli` — observer rendering: surfaces `StrategicBudgetTrace` in Section 9 (Budget Exhaustion Snapshots) and `PlanningStateCacheCounters` aggregates wherever the existing `SnapshotCacheCounters` are rendered.
- `worldwake-sim` — no change.
- `worldwake-systems` — no change.

## Dependencies

- S88 (Two-Phase Landmark Planning, archived at `archive/specs/S88-two-phase-landmark-planning.md`) — provides the two-phase strategic + tactical search the budget feeds.
- S89 (Universal Two-Phase Planning, archived at `archive/specs/S89-universal-two-phase-planning.md`) — provides `TravelToGoal` tactical scoping that interacts with strategic stage emission.
- S132 (Frontier-Exhaustion Strategy as Goal-Kind Property, archived at `archive/specs/S132-frontier-exhaustion-strategy.md`) — provides the frontier-exhaustion dispatch table that strategic search consults.
- S144 (Aggregate Scenario Diagnostics, archived at `archive/specs/S144-aggregate-scenario-diagnostics.md`) — shipped `SnapshotCacheCounters` (`crates/worldwake-ai/src/decision_trace.rs:107`) for `PlanningSnapshot` distance-matrix caches and `PerformanceMetrics.cache_hit_count`/`cache_miss_count`/`cache_invalidation_count` in `ScenarioDiagnosticsReport`. S145 extends the diagnostics surface additively with `PlanningStateCacheCounters` covering the separate `PlanningState` cache substrate.

## Design Goals

1. **Strategic budget scales with stage depth.** Multi-prerequisite acquisition chains receive proportional expansion budget.
2. **Strategic exhaustion is diagnosable.** `StrategicBudgetTrace` on `PlanAttemptTrace` makes "did the chain bust the strategic budget?" attributable per attempt while strategic search still returns `None` on budget exhaustion until S149's typed terminal work.
3. **Cache invariants are tested, not just commented.** A compound-order regression test proves sibling-branch order-independence for the planning-state caches across the full mutation surface, complementing the existing single-mutation cross-clone tests.
4. **No new authoritative state.** Both fixes are local to search machinery; counters are pure observability per FND-27.
5. **Backward-compat-free.** No legacy formula path; the old private helper and its test are deleted, not aliased, per FND-28.

## Non-Goals

- **No typed `BudgetExhausted` strategic terminal.** Strategic search continues to return `Option<StrategicPlan>`. Routing strategic exhaustion through a typed terminal that the planner reacts to is S149's typed-plan-terminal scope.
- **No replacement of `Rc<RefCell<...>>` with a different concurrency primitive.** The proposal in the assessment ("replace shared mutable caches with immutable per-node memo tables") is rejected as premature optimization. S145 only proves the current shape is correct.
- **No incremental snapshot mechanism.** That would address PR-9 (rejected for this phase, reassessed post-S144).
- **No per-goal budget override.** That is S146's deliverable.
- **No change to `beam_width`, `max_node_expansions`, or `max_plan_depth` defaults.** Those are tuned by S146.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-12 (Performance May Compress Computation, Never Causality) | Strategic budget change preserves causal completeness — longer chains either complete or terminate, world meaning is unchanged. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Cache counters and trace fields are read-only observability; no system-to-system mutation introduced. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | The cache invariant test and module-level documentation literally enforce this principle for `entities_at_cache` and `effective_place_cache`; `PlanningStateCacheCounters` is per-run derived read-model state, never authoritative. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The private `strategic_search_budget` helper and its unit test are deleted in the same change that introduces `ExecutionBudget::strategic_budget_for_stages`. No dual-truth window. |
| FND-29 (Debuggability Is a Product Feature) | `StrategicBudgetTrace` carries stage-count, used-vs-total budget, and exhaustion provenance so observer Section 9 and S144's diagnostics aggregator can attribute exhaustion to chain depth rather than guessing. |

## Deliverables

### D1: Stage-aware strategic budget

```rust
// crates/worldwake-core/src/execution_budget.rs (new method on existing impl)
impl ExecutionBudget {
    pub const fn strategic_budget_for_stages(&self, stage_count: usize) -> usize {
        // Treat zero stages as one to preserve the single-stage default
        // expansion budget (with default `max_prerequisite_locations = 3`,
        // a single-stage search receives 2 * 1 * 3 = 6 expansions, identical
        // to the pre-S145 formula). No `usize::max(1, ...)` defensive floor
        // is needed because `ExecutionBudget::try_new` rejects
        // `max_prerequisite_locations == 0`.
        let stages = if stage_count == 0 { 1 } else { stage_count };
        2 * stages * self.max_prerequisite_locations() as usize
    }
}
```

```rust
// crates/worldwake-ai/src/search/strategic.rs — replace the call at L113
let search_budget = execution_budget.strategic_budget_for_stages(stages.len());
```

**Deletion (FND-28 single-truth)**: Delete the private `strategic_search_budget` helper at `crates/worldwake-ai/src/search/strategic.rs:167-175` (now unreachable) and its accompanying unit test `strategic_search_budget_tracks_execution_budget_stage_cap` at `crates/worldwake-ai/src/search/strategic.rs:1004-1017`. Migrate the unit test into `crates/worldwake-core/src/execution_budget.rs` covering the new `strategic_budget_for_stages` method.

The default `max_prerequisite_locations = 3` becomes:
- 1 stage → 6 (unchanged from today)
- 3 stages → 18
- 5 stages → 30
- 8 stages (PR-2 typical production chain) → 48

Single-stage goals (`Sleep`, `Eat`, `Drink`, `Relieve`, `Wash`) see identical budget. Multi-stage chains (production, restocking, bounty fulfillment) see proportional budget growth.

### D2: Strategic budget exhaustion provenance

`PlanAttemptTrace` (existing at `crates/worldwake-ai/src/decision_trace.rs:1121`) gains a new optional field carrying strategic-search outcome data:

```rust
// crates/worldwake-ai/src/decision_trace.rs (new type alongside StrategicStepTrace)
#[derive(Clone, Debug)]
pub struct StrategicBudgetTrace {
    pub stages_count: u16,
    pub budget_total: u32,
    pub budget_used: u32,
    pub exhausted: bool,
}

pub struct PlanAttemptTrace {
    // ...existing fields unchanged...
    pub strategic_budget: Option<StrategicBudgetTrace>,
}
```

`Clone, Debug` derives mirror the existing `PlanAttemptTrace` and `StrategicStepTrace` shape. The field is `Option<_>` because not every plan attempt enters the two-phase strategic path.

The strategic search at `crates/worldwake-ai/src/search/strategic.rs` populates the trace from local counters at the existing `break`/return sites (lines 124–131). No change to strategic search's return type (`Option<StrategicPlan>`) — see Non-Goals.

**Observer rendering**: The `StrategicBudgetTrace` is surfaced in `crates/worldwake-cli/src/bin/observer.rs` Section 9 (Budget Exhaustion Snapshots) at line 1076, alongside the existing per-snapshot `max_prerequisite_locations` render (line 1134). This makes "did the chain bust the strategic budget?" diagnosable per attempt rather than inferred from tactical thrash. A future S144 extension can also aggregate strategic-budget exhaustion-by-stage-count into `PlanningMetrics`; this spec does not modify S144's already-shipped surface beyond the additive read.

### D3: Cache compound-order regression

`crates/worldwake-ai/src/planning_state.rs` already covers single-mutation cross-clone independence in two tests:

- `entities_at_cache_is_invalidated_when_holder_moves_across_branches` (`:4179`)
- `effective_place_cache_is_invalidated_when_holder_moves_across_branches` (`:4210`)

These prove that cloning a state and mutating the clone does not corrupt the base state's cache. They do **not** prove compound-order independence: that applying the same mutation set in different orders on sibling branches produces the same cache state.

D3 adds **one new test** covering the compound-order invariant across the full mutation surface (`move_lot_ref_to_holder`, `move_lot_ref_to_ground`, `move_entity_ref`, `set_possessor_ref`, `set_container_ref`, `mark_removed_ref`):

```rust
#[test]
fn cache_results_are_order_independent_across_sibling_branches() {
    // Build a PlanningState with two believed entities and a hypothetical lot.
    // Apply mutation sequence A then B on one cloned branch, B then A on
    // another cloned branch. For every place and entity queried, both
    // branches must return equal `entities_at` and `effective_place` results
    // after the final mutation. Exercises movement, possession-change, and
    // removal mutators in both orderings.
}
```

The compound-order test is the genuinely net-new coverage. D3 does not add an `effective_place_cache_invalidates_on_move` test — that case is already covered by the existing `effective_place_cache_is_invalidated_when_holder_moves_across_branches` at `:4210`.

A second new test exists if and only if D4 lands counters — see D3.5 below.

### D3.5: Cache counter invariant test (depends on D4)

```rust
#[test]
fn cache_invalidation_count_increments_on_each_mutation() {
    // Surface PlanningStateCacheCounters; assert .invalidations advances by
    // exactly 1 per mutation across all six invalidating mutators
    // (move_lot_ref_to_holder, move_lot_ref_to_ground, move_entity_ref,
    // set_possessor_ref, set_container_ref, mark_removed_ref) and does NOT
    // advance for set_quantity_ref (which deliberately does not invalidate).
}
```

This is bundled with D3 in implementation, but logically belongs to D4's counter surface.

### D4: Planning-state cache hit/miss/invalidation counters

The already-shipped `SnapshotCacheCounters` (`crates/worldwake-ai/src/decision_trace.rs:107`) covers `PlanningSnapshot`'s precomputed `DistanceMatrix` caches and is surfaced through `AgentDecisionTrace.snapshot_cache_counters` and aggregated into `PerformanceMetrics.cache_hit_count`/`cache_miss_count`/`cache_invalidation_count`. `PlanningState`'s `entities_at_cache` and `effective_place_cache` are a *separate* cache substrate that S144 did not cover.

D4 introduces a parallel runtime counter type with per-cache breakdown:

```rust
// crates/worldwake-ai/src/decision_trace.rs (new type alongside SnapshotCacheCounters)
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
    pub fn is_empty(self) -> bool { /* mirrors SnapshotCacheCounters::is_empty */ }
    pub fn add_assign(&mut self, other: Self) { /* mirrors SnapshotCacheCounters::add_assign */ }
}
```

Derives mirror the existing `SnapshotCacheCounters` (`Clone, Copy, Debug, Default, Eq, PartialEq`). The per-cache breakdown (vs. S144's flat single-counter shape) is intentional because the two `PlanningState` caches answer different queries and may have different hit ratios — collapsing them would lose diagnostic signal.

**Storage**: `PlanningState` accumulates counters as it serves reads and as `invalidate_entities_at_cache` runs. Counters are zero-initialized in `PlanningState::new` and never mutate authoritative state.

**Surfacing**: `AgentDecisionTrace` gains an `Option<PlanningStateCacheCounters>` field alongside the existing `snapshot_cache_counters`. The aggregator (S144's `build_scenario_diagnostics`) extends `PerformanceMetrics` *additively* with new fields:

```rust
// archive/specs/S144 PerformanceMetrics surface (extension — S145 deliverable)
pub struct PerformanceMetrics {
    // ...existing S144 fields unchanged...
    pub planning_state_cache_entities_at_hits: u64,
    pub planning_state_cache_entities_at_misses: u64,
    pub planning_state_cache_effective_place_hits: u64,
    pub planning_state_cache_effective_place_misses: u64,
    pub planning_state_cache_invalidations: u64,
}
```

The existing `cache_hit_count`/`cache_miss_count`/`cache_invalidation_count` fields are **not** repurposed — they continue to report only `SnapshotCacheCounters`. S144's already-shipped fixture (`crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json`) is regenerated to include the new fields at known-deterministic values.

No correctness change — pure observability per FND-27.

### D5: Documented cache invariant

A module-level doc comment on `crates/worldwake-ai/src/planning_state.rs` documenting:

- The `Rc<RefCell<...>>` caches at lines 71–72 are *memoization only*; they cache pure functions of `PlanningState`'s mutable substrate (place overrides, possessor overrides, container overrides, removed-entity set).
- Any mutation that could change the cached function output must call `invalidate_entities_at_cache` before the read path can observe stale data. The six mutators that currently invalidate are listed in the module doc as the authoritative invariant surface.
- Sibling search branches that mutate state in different orders must produce equal cache outputs (D3 enforces this).
- The cache must never be promoted to source of truth (FND-27).

## FND-01 Section H Analysis

### Information-Path Analysis

Not applicable. S145 modifies search-space exploration budget and adds cache observability; no new world-information flow.

### Positive-Feedback Analysis

Not applicable. No new amplifying loops.

### Concrete Dampeners

The stage-aware strategic budget *itself* is a dampener: it bounds search expansion per stage count, preventing unbounded permutation growth on deep chains. The bound is concrete: `2 * stages * max_prerequisite_locations`. Per FND-11, this is acceptable because the dampener is the strategic search budget, not a numeric clamp on world state.

### Stored State vs. Derived Read-Model List

**Stored state**: `PlanningStateCacheCounters` is per-`PlanningState` runtime state (not persisted across ticks, not save-loaded). Not authoritative world state.

**Derived read-model**: `entities_at_cache` and `effective_place_cache` remain derived per FND-27; D5 documents this. `StrategicBudgetTrace` on `PlanAttemptTrace` is derived from local strategic-search counters and consumed read-only by observer rendering and (additively) by S144's aggregator.

## SystemFn Integration

Not applicable. S145 introduces no new `SystemFn`.

## Component Registration

Not applicable. S145 introduces no new ECS component.

## Cross-System Interactions

- Strategic search reads `ExecutionBudget` (already does today) and uses the new `strategic_budget_for_stages` method.
- `AgentDecisionTrace` carries `Option<PlanningStateCacheCounters>` alongside the existing `Option<SnapshotCacheCounters>`.
- Observer reads `StrategicBudgetTrace` and `PlanningStateCacheCounters` through the trace surfaces it already consumes.
- S144's aggregator extends `PerformanceMetrics` additively with `planning_state_cache_*` fields.

No new cross-system mutation paths.

## Profile-Driven Parameters

`ExecutionBudget.max_prerequisite_locations` is already per-agent (via `CognitiveProfile`). The new `strategic_budget_for_stages` method consumes it. No new profile parameter is introduced.

## Test Plan

- D1 unit tests (in `crates/worldwake-core/src/execution_budget.rs`): assert `strategic_budget_for_stages(0) == 6`, `strategic_budget_for_stages(1) == 6`, `strategic_budget_for_stages(3) == 18`, `strategic_budget_for_stages(5) == 30`, `strategic_budget_for_stages(8) == 48` (default `max_prerequisite_locations = 3`).
- D3 compound-order regression test (single new test as above).
- D3.5 cache counter invariant test (depends on D4 landing first).
- Existing `golden_two_phase_planning.rs` and `golden_planner_pathology.rs` regress unchanged (no behavioral change on single-stage paths).
- New `golden_strategic_budget_scaling.rs` proves a 5-stage production chain completes under the new budget where it timed out under the old `* 2` formula.
- S144's fixture `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` is regenerated to include the new `planning_state_cache_*` fields.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
