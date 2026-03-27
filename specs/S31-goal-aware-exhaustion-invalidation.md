**Status**: PENDING

# S31: Goal-Aware Exhaustion Invalidation

## Summary

Replace the coarse dirty-bit mask for exhaustion cache invalidation with per-goal invalidation conditions. Currently, ANY commodity change clears ALL exhausted goals (even unrelated ones), and NEEDS changes never clear exhausted goals (even when a need is the reason the goal is unsolvable). This spec defines a mechanism where each exhausted goal records the specific world conditions that would make it worth re-searching, and the cache clears only when those conditions change.

## Why This Exists

The golden-perf campaign identified two manifestations of the coarse invalidation problem:

1. **Over-invalidation (COMMODITY)**: Agent eats bread -> COMMODITY dirty bit fires -> ALL exhausted goals cleared, including `AcquireCommodity(Apple)` which has nothing to do with bread. This caused 52 redundant full-budget re-searches per 200-tick scenario (3s of wasted computation at budget=256). The TTL-based skip (EXHAUSTION_SKIP_TTL=16) mitigates this but does not eliminate it.

2. **Under-invalidation (NEEDS)**: NEEDS is excluded from the invalidation mask ("needs-only changes rarely alter search space"). But for goals like `Wash` (dirtiness-driven), the search space changes when dirtiness crosses a threshold. exp-005 showed that indefinitely caching exhausted goals caused 4 golden test failures because agents never re-evaluated goals that became solvable as needs changed.

The current TTL-based skip is a compromise: it periodically re-searches everything, wasting time on genuinely unsolvable goals while also delaying detection of newly solvable ones. Goal-aware invalidation solves both problems by clearing only when relevant conditions change.

## Phase

Phase 3+: AI Architecture Overhaul

## Crates

- `worldwake-ai`

## Dependencies

- S30 (AI Runtime Save/Load Parity) — the invalidation conditions must survive save/load if the exhaustion cache does.
- S29 (Planning State Structural Sharing) — recommended but not required; reduces the cost of re-searches that do occur after invalidation.

## Design Goals

1. **Precise invalidation**: Each exhausted goal's cache entry specifies which world conditions would make it worth re-searching. The cache clears for a goal only when those conditions change.
2. **No over-invalidation**: Bread consumption does not clear the Apple acquisition cache. Position changes at waypoints do not clear travel-irrelevant caches.
3. **No under-invalidation**: Dirtiness increase clears the Wash exhaustion cache when dirtiness crosses the relevant threshold. Resource regeneration at a known source clears acquisition caches for that commodity.
4. **Profile-driven, not magic numbers**: Invalidation conditions are derived from the goal's `GoalKind` and the agent's belief state, not from hardcoded rules (Principle 2).
5. **Determinism preserved**: Invalidation is a pure function of world state changes, not of timing or order of evaluation.

## Deliverables

### 1. `ExhaustionInvalidationCondition` enum

```rust
/// Condition that would make a previously-exhausted goal worth re-searching.
/// Derived from the goal's GoalKind and the agent's state at exhaustion time.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ExhaustionInvalidationCondition {
    /// Agent's effective position changes (settles at a new place).
    PositionChanged,
    /// Quantity of a specific commodity changes for the agent.
    CommodityChanged(CommodityKind),
    /// Agent acquires or loses a unique item of a specific kind.
    UniqueItemChanged(UniqueItemKind),
    /// Wound state changes (new wound, wound healed).
    WoundsChanged,
    /// Facility access changes at the agent's current place.
    FacilitiesChanged,
    /// A blocked intent expires (blocker cleanup).
    BlockerExpired,
    /// A specific need crosses a threshold (for needs-driven goals).
    NeedCrossedThreshold {
        need: NeedKind,
        /// The need level at exhaustion time. Re-search when the
        /// absolute difference exceeds this delta.
        threshold_delta: Permille,
    },
    /// Resource quantity at a known source changes (for acquisition goals).
    ResourceAtSourceChanged(EntityId),
}

/// Which homeostatic need dimension.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum NeedKind {
    Hunger,
    Thirst,
    Fatigue,
    Bladder,
    Dirtiness,
}
```

### 2. `ExhaustionEntry` struct

Replace `BTreeMap<GoalKey, Tick>` with `BTreeMap<GoalKey, ExhaustionEntry>`:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExhaustionEntry {
    /// Tick when the goal was marked exhausted.
    pub exhausted_at: Tick,
    /// Conditions that would warrant re-searching this goal.
    pub invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
    /// Snapshot of relevant state at exhaustion time (for delta detection).
    pub baseline: ExhaustionBaseline,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExhaustionBaseline {
    pub position: Option<EntityId>,
    pub needs: Option<HomeostaticNeeds>,
    // Only populated for commodity-dependent goals:
    pub relevant_commodity_quantities: Vec<(CommodityKind, Quantity)>,
}
```

### 3. `derive_invalidation_conditions` function

Maps `GoalKind` to the set of conditions that would make the goal's search space materially different:

```rust
fn derive_invalidation_conditions(
    goal: &GoalKind,
    agent: EntityId,
    view: &dyn RuntimeBeliefView,
    recipe_registry: &RecipeRegistry,
) -> Vec<ExhaustionInvalidationCondition> {
    // Example mappings:
    // AcquireCommodity(Apple, _) -> [PositionChanged, CommodityChanged(Apple)]
    // ConsumeOwnedCommodity(Water) -> [CommodityChanged(Water)]
    // Wash -> [NeedCrossedThreshold { Dirtiness, delta: 100 }, FacilitiesChanged]
    // ProduceCommodity(RecipeId) -> [PositionChanged, FacilitiesChanged, inputs...]
    // RestockCommodity(Apple) -> [PositionChanged, CommodityChanged(Apple), CommodityChanged(Coin)]
    // HealWounds -> [WoundsChanged, CommodityChanged(Medicine)]
    // ShareBelief -> [PositionChanged] (different listeners at different places)
    // PoliticalGoals -> [PositionChanged, BlockerExpired]
}
```

### 4. Replace `reset_exhausted_goals_if_needed` with `invalidate_exhausted_goals`

Instead of checking a single dirty-bit mask and clearing ALL goals, iterate the exhaustion map and check each goal's specific conditions against the current observation snapshot:

```rust
fn invalidate_exhausted_goals(
    runtime: &mut AgentDecisionRuntime,
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
) {
    runtime.search_exhausted_goals.retain(|_goal, entry| {
        // Keep the entry (don't invalidate) if none of the conditions changed.
        !entry.invalidation_conditions.iter().any(|condition| {
            condition_changed(condition, &entry.baseline, view, agent)
        })
    });
}
```

### 5. Remove EXHAUSTION_SKIP_TTL

With precise invalidation, the TTL-based periodic re-search is no longer needed. Goals stay cached until their specific conditions change. The TTL constant and the tick-based filtering in `build_candidate_plans` are removed.

### 6. Update `record_exhausted_goals` to capture conditions

When a goal exhausts the search budget, derive its invalidation conditions and baseline, then store them in the exhaustion map.

## Component Registration

No new ECS components. `ExhaustionEntry` is AI-layer runtime state, not world state.

If S30 (save/load parity) is implemented, `ExhaustionEntry` must be serializable (already has `Serialize`/`Deserialize` derives).

## SystemFn Integration

### `worldwake-ai`

- Replace `reset_exhausted_goals_if_needed` in `planning.rs` with `invalidate_exhausted_goals`.
- Replace `BTreeMap<GoalKey, Tick>` in `AgentDecisionRuntime` with `BTreeMap<GoalKey, ExhaustionEntry>`.
- Add `derive_invalidation_conditions` in a new `exhaustion.rs` module or within `goal_model.rs`.
- Remove `EXHAUSTION_SKIP_TTL` constant from `planning.rs`.
- Remove the tick-based filtering logic in `build_candidate_plans`.

## Cross-System Interactions (Principle 12)

No cross-system interactions. The invalidation conditions read from the existing `RuntimeBeliefView` interface (which already abstracts over world state). No system calls another system's logic. The conditions are derived from the goal's `GoalKind` semantics, not from querying other systems directly.

## FND-01 Section H

### H.1 Information-Path Analysis

Exhaustion invalidation reads the same belief state that candidate generation already reads. No new information paths are introduced. The conditions (`PositionChanged`, `CommodityChanged`, etc.) are derived from comparing the current belief snapshot against the baseline captured at exhaustion time — both obtained through the existing `RuntimeBeliefView`.

### H.2 Positive-Feedback Analysis

**No amplifying loops introduced.** The invalidation system is purely reactive: world state changes -> condition check -> cache clear or retain. It does not generate actions, modify world state, or influence other agents.

Potential concern: could precise invalidation cause a goal to oscillate between "exhausted" and "not exhausted"? Yes, but this is not amplifying — it is the correct response to a changing world. The exponential backoff on `exhaustion_counts` dampens the computational cost of re-searches (Principle 10).

### H.3 Concrete Dampeners

The exponential backoff mechanism (`exhaustion_counts`) serves as the dampener for re-search oscillation. Each time a goal exhausts again after invalidation, its budget is halved (256 -> 128 -> 64). After 3 exhaustions, the search cost is 87.5% reduced. This is a physical world mechanism (bounded computation budget) not a numeric clamp.

### H.4 Stored vs Derived State

- **Stored (in runtime, serialized with S30)**: `ExhaustionEntry` (invalidation conditions + baseline snapshot + exhaustion tick).
- **Derived (not stored)**: The invalidation CHECK (comparison of current state against baseline) is a transient computation performed at the start of each planning tick. The result is a boolean (invalidated or not) that is never stored.
- **Source of truth**: The exhaustion conditions are derived from `GoalKind` semantics (which are immutable constants) and the agent's belief state at exhaustion time (captured in the baseline). Neither is an abstract score — both are concrete references to world state (Principle 3).

## Invariants

1. A goal is invalidated (re-searched) if and only if at least one of its recorded conditions has changed since the baseline was captured.
2. `derive_invalidation_conditions` is a pure function of `GoalKind` + agent belief state. Given the same inputs, it produces the same conditions.
3. The exhaustion map is deterministic: same world state history -> same map contents.
4. No goal is permanently cached — conditions that reference world state will eventually be invalidated by world evolution. (Unlike the TTL approach, this is semantically correct rather than time-based.)
5. The system degrades gracefully to "re-search everything" when `derive_invalidation_conditions` returns an empty list (conservative fallback for unknown goal kinds).

## Tests

- [ ] Unit test: `derive_invalidation_conditions` for each GoalKind returns non-empty conditions.
- [ ] Unit test: `condition_changed` correctly detects position/commodity/wound/facility changes.
- [ ] Golden test: Agent with exhausted `AcquireCommodity(Apple)` does NOT re-search when bread is consumed (no over-invalidation).
- [ ] Golden test: Agent with exhausted `Wash` goal re-searches when dirtiness crosses threshold (no under-invalidation).
- [ ] Golden test: `golden_save_load_round_trip_under_ai` passes without driver reset (requires S30).
- [ ] Golden test: `golden_wash_action` passes (the test that broke in exp-005 due to indefinite caching).
- [ ] Golden test: `golden_three_way_need_competition` passes (the test that broke in exp-005).
- [ ] Profiling: fewer total exhausted re-searches than TTL=16 approach on `golden_world_runs_without_observers`.
- [ ] All 2700+ workspace tests pass.

## Acceptance Criteria

1. EXHAUSTION_SKIP_TTL is removed entirely — invalidation is condition-based, not time-based.
2. No over-invalidation: irrelevant commodity changes do not trigger re-searches.
3. No under-invalidation: needs-driven goals re-search when the relevant need crosses a threshold.
4. All golden tests pass (including the 4 that broke in exp-005 with indefinite caching).
5. Profiling shows fewer total exhausted searches than TTL=16 on the golden_world_runs_without_observers test.
6. Per-goal invalidation conditions are documented and testable.

## References

- golden-perf campaign: exp-005 (indefinite caching broke 4 tests), exp-013/014/015 (TTL tuning), exp-016 (TTL=32 too aggressive)
- `crates/worldwake-ai/src/agent_tick/planning.rs` — `reset_exhausted_goals_if_needed`, `record_exhausted_goals`, `EXHAUSTION_SKIP_TTL`
- `crates/worldwake-ai/src/decision_runtime.rs` — `search_exhausted_goals`, `exhaustion_counts`
- `crates/worldwake-ai/src/goal_model.rs` — `GoalKind` and `GoalKindPlannerExt`
- `docs/FOUNDATIONS.md` Principles 2, 3, 10, 11, 18, 19, 24, 25
