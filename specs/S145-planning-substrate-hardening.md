# S145: Planning Substrate Hardening

**Status**: Draft

## Summary

The external assessment in `reports/ai-architecture-improvements.md` flags two narrow planner-substrate concerns that the rest of the assessment's larger architectural proposals depend on:

1. **Strategic search budget collapses on long acquisition chains** (PR-16): The current formula at `crates/worldwake-ai/src/search/strategic.rs:172` is `usize::from(execution_budget.max_prerequisite_locations()) * 2`. With the default `max_prerequisite_locations = 3` (`crates/worldwake-core/src/execution_budget.rs:6`), every strategic search gets 6 expansions regardless of how many stages the goal decomposes into. A 5-stage production chain gets the same budget as a 1-stage hunger goal, which means longer chains exhaust the strategic budget at the first wave of permutations and never reach the deeper stage permutations.

2. **Shared planning caches lack a determinism regression** (PR-18): `crates/worldwake-ai/src/planning_state.rs:69-70` uses `Rc<RefCell<BTreeMap<...>>>` for `entities_at_cache` and `effective_place_cache`. The invalidation hooks at lines 104-107 reset the cache on `move_entity*` mutations, but there is no regression test asserting that two sibling search branches that mutate state in different orders see consistent cache results. For a deterministic simulation, accidental cache order dependence is silently corruptive.

S145 lands both fixes as small, narrowly-scoped architectural improvements. The strategic-budget fix multiplies by `stages.len()` so the budget scales with decomposition depth. The cache fix adds a regression-test suite plus a documented invariant in code comments so the cache shape is reviewable and the order-independence claim is enforceable.

This spec is intentionally narrow. It is a pre-requisite for S146 (Goal Schema and Per-Goal Budgets) — which will further refine these surfaces — but ships separately because (a) it has no dependency on S146's larger registry refactor and (b) it gives S146 a stable substrate to build on.

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — owns the strategic-budget formula change in `search/strategic.rs` and the planning-state cache invariant tests in `planning_state.rs`.
- `worldwake-core` — exposes `ExecutionBudget` accessor for a new `strategic_budget_for_stages(stage_count: usize) -> usize` helper so the formula change is testable without reaching into AI-crate internals.
- `worldwake-sim` — no change.
- `worldwake-systems` — no change.
- `worldwake-cli` — no change.

## Dependencies

- S88 (Two-Phase Landmark Planning, archived) — provides the two-phase strategic + tactical search the budget feeds.
- S89 (Universal Two-Phase Planning, archived) — provides `TravelToGoal` tactical scoping that interacts with strategic stage emission.
- S132 (Frontier-Exhaustion Strategy as Goal-Kind Property, archived) — provides the frontier-exhaustion dispatch table that strategic search consults.

## Design Goals

1. **Strategic budget scales with stage depth.** Multi-prerequisite acquisition chains receive proportional expansion budget.
2. **No silent failure on chain length.** The current behavior — strategic search returns a partial itinerary that thrashes in tactical — becomes explicit as either a successful longer-budget search or a deterministic `BudgetExhausted` terminal that the planner can react to.
3. **Cache invariants are tested, not just commented.** Three regression tests prove sibling-branch order-independence for both caches.
4. **No new authoritative state.** Both fixes are local to search machinery.
5. **Backward-compat-free.** No legacy formula path; the old `* 2` constant is replaced, not shimmed.

## Non-Goals

- **No replacement of `Rc<RefCell<...>>` with a different concurrency primitive.** The proposal in the assessment ("replace shared mutable caches with immutable per-node memo tables") is rejected as premature optimization. S145 only proves the current shape is correct.
- **No incremental snapshot mechanism.** That would address PR-9 (rejected for this phase, reassessed post-S144).
- **No per-goal budget override.** That is S146's deliverable.
- **No change to `beam_width`, `max_node_expansions`, or `max_plan_depth` defaults.** Those are tuned by S146.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-12 (Performance May Compress Computation, Never Causality) | Strategic budget change preserves causal completeness — longer chains either complete or terminate with a typed barrier; world meaning is unchanged. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Cache invariants test reads only; no system mutations introduced. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | The cache invariant tests literally enforce this principle for `entities_at_cache` and `effective_place_cache`. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | Old strategic budget formula is replaced, not aliased. |
| FND-29 (Debuggability Is a Product Feature) | Budget-exhaustion terminals carry stage-count provenance so the diagnostics report (S144) can attribute exhaustion to chain depth. |

## Deliverables

### D1: Stage-aware strategic budget

```rust
// crates/worldwake-core/src/execution_budget.rs
impl ExecutionBudget {
    pub fn strategic_budget_for_stages(&self, stage_count: usize) -> usize {
        // Formula: 2 * stages * max_prerequisite_locations.
        // Floor at 2 to preserve single-stage default behavior.
        let stages = stage_count.max(1);
        2 * stages * usize::from(self.max_prerequisite_locations())
    }
}
```

```rust
// crates/worldwake-ai/src/search/strategic.rs:166-173 (replaced)
let strategic_budget = execution_budget.strategic_budget_for_stages(stages.len());
```

The default `max_prerequisite_locations = 3` becomes:
- 1 stage → 6 (unchanged from today)
- 3 stages → 18
- 5 stages → 30
- 8 stages (PR-2 typical production chain) → 48

Single-stage goals (`Sleep`, `Eat`, `Drink`, `Relieve`, `Wash`) see identical budget. Multi-stage chains (production, restocking, bounty fulfillment) see proportional budget growth.

### D2: Strategic budget exhaustion provenance

`PlanAttemptTrace` (existing in `decision_trace.rs`) gains a new field:

```rust
pub struct StrategicBudgetTrace {
    pub stages_count: usize,
    pub budget_used: usize,
    pub budget_total: usize,
    pub exhausted: bool,
}
```

Surfaced in observer Section 7 and in S144's `PlanningMetrics`. This makes "did the chain bust the strategic budget?" diagnosable per attempt rather than inferred from tactical thrash.

### D3: Cache invariant regression suite

Three tests in `crates/worldwake-ai/src/planning_state.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn entities_at_cache_is_order_independent_across_sibling_branches() {
    // Build PlanningState with two believed entities, mutate move_entity_to(...)
    // along branch A then branch B in opposite order on cloned states; assert
    // both states' entities_at_cache reads return equal results for every
    // queried place after the same mutation sequence (regardless of order).
}

#[test]
fn effective_place_cache_invalidates_on_move() {
    // Pre-warm cache, move entity, query effective_place — must return new place
    // not cached old place.
}

#[test]
fn cache_invalidation_count_increments_on_each_move() {
    // Surface cache_invalidation_count counter (D4); ensure it advances per
    // mutation that resets the cache.
}
```

### D4: Cache hit/miss/invalidation counters

```rust
// crates/worldwake-ai/src/planning_state.rs (added)
pub struct PlanningCacheCounters {
    pub entities_at_hits: u64,
    pub entities_at_misses: u64,
    pub effective_place_hits: u64,
    pub effective_place_misses: u64,
    pub invalidations: u64,
}
```

`PlanningState` accumulates counters as it serves reads and as it invalidates. The counters are surfaced through `PerfTelemetry` (per S144's D7) and into `ScenarioDiagnosticsReport.performance.cache_*`. No correctness change — pure observability.

### D5: Documented cache invariant

A module-level doc comment on `planning_state.rs` documenting:
- The `Rc<RefCell<...>>` caches are *memoization only*; they cache pure functions of `PlanningState`'s mutable substrate.
- Any mutation that could change the cached function output must call `invalidate_entities_at_cache` before the read path can observe stale data.
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

**Stored state**: `PlanningCacheCounters` is per-`PlanningState` runtime state (not persisted across ticks, not save-loaded). Not authoritative world state.

**Derived read-model**: `entities_at_cache` and `effective_place_cache` remain derived per FND-27; D5 documents this.

## SystemFn Integration

Not applicable. S145 introduces no new `SystemFn`.

## Component Registration

Not applicable. S145 introduces no new ECS component.

## Cross-System Interactions

- Strategic search reads `ExecutionBudget` (already does today) and uses the new helper.
- `PerfTelemetry` reads cache counters (per S144's D7).
- Observer reads strategic-budget traces and cache counters.

No new cross-system mutation paths.

## Profile-Driven Parameters

`ExecutionBudget.max_prerequisite_locations` is already per-agent (via `CognitiveProfile`). The strategic-budget formula consumes it. No new profile parameter is introduced.

## Test Plan

- D1 unit tests: assert `strategic_budget_for_stages(1) == 6`, `strategic_budget_for_stages(5) == 30`, etc.
- D3 cache regression tests (3 above).
- Existing `golden_two_phase_planning.rs` and `golden_planner_pathology.rs` regress unchanged (no behavioral change on single-stage paths).
- New `golden_strategic_budget_scaling.rs` proves a 5-stage production chain completes under the new budget where it timed out under the old `* 2` formula.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
