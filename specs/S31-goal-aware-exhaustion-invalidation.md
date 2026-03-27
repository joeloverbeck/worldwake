**Status**: PENDING

# S31: Goal-Aware Exhaustion Invalidation

## Summary

Replace the coarse dirty-bit mask for exhaustion cache invalidation with per-goal invalidation conditions. Currently, ANY commodity change clears ALL exhausted goals (even unrelated ones), and NEEDS changes never clear exhausted goals (even when a need is the reason the goal is unsolvable). This spec defines a mechanism where each exhausted goal records the specific world conditions that would make it worth re-searching, and the cache clears only when those conditions change.

## Why This Exists

The golden-perf campaign identified two manifestations of the coarse invalidation problem:

1. **Over-invalidation (COMMODITY)**: Agent eats bread -> COMMODITY dirty bit fires -> ALL exhausted goals cleared, including `AcquireCommodity(Apple)` which has nothing to do with bread. This caused 52 redundant full-budget re-searches per 200-tick scenario (3s of wasted computation at budget=256). The TTL-based skip mitigates this but does not eliminate it. The live code uses `EXHAUSTION_SKIP_TTL = 20`; [archive/tickets/S30-007-increase-exhaustion-ttl.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S30-007-increase-exhaustion-ttl.md) raised the value from 16 to 20, the highest value proven safe by `golden_save_load_round_trip_under_ai`.

2. **Under-invalidation (NEEDS)**: NEEDS is excluded from the invalidation mask ("needs-only changes rarely alter search space"). But for goals like `Wash` (dirtiness-driven), the search space changes when dirtiness crosses a threshold. exp-005 showed that indefinitely caching exhausted goals caused 4 golden test failures because agents never re-evaluated goals that became solvable as needs changed.

The current TTL-based skip is a compromise: it periodically re-searches everything, wasting time on genuinely unsolvable goals while also delaying detection of newly solvable ones. Goal-aware invalidation solves both problems by clearing only when relevant conditions change.

## Assumption Reassessment (2026-03-27)

1. The live exhaustion runtime stores a unified `ExhaustionEntry { exhausted_at: Option<Tick>, count: u8 }` in `BTreeMap<GoalKey, ExhaustionEntry>` at `AgentDecisionRuntime.exhaustion_cache` (`crates/worldwake-ai/src/decision_runtime.rs:55-84`). The `ExhaustionEntry` currently derives `Copy`; S31 must remove `Copy` since the new fields contain `Vec`.
2. The live planner uses `EXHAUSTION_SKIP_TTL = 20` in `crates/worldwake-ai/src/agent_tick/planning.rs:21`.
3. Live verification during S30-007 established that `golden_save_load_round_trip_under_ai` fails at `EXHAUSTION_SKIP_TTL >= 21`, confirming the coarse TTL architecture has a hard determinism ceiling that S31 removes.
4. `HomeostaticNeedId` already exists in `crates/worldwake-core/src/needs.rs:18-25` with variants `Hunger`, `Thirst`, `Fatigue`, `Bladder`, `Dirtiness`. No new `NeedKind` enum is needed.
5. `GoalKind` currently has 23 variants (as of E17 completion). All must be covered by `derive_invalidation_conditions`.
6. `SAVE_FORMAT_VERSION` is currently 6 (`crates/worldwake-sim/src/save_load.rs:6`). Adding `Vec` fields to `ExhaustionEntry` changes the bincode format.
7. The `reset_exhausted_goals_if_needed` function (`planning.rs:311-348`) uses `DirtySet::COMMODITY | UNIQUE_ITEMS | WOUNDS | FACILITIES | REPLAN_SIGNAL | BLOCKER_CLEANUP` as the non-position invalidation mask. NEEDS is intentionally excluded.
8. The exponential backoff in `build_candidate_plans` (`planning.rs:214-223`) halves the search budget per `entry.count`, capped at 3 halvings (floor 64 expansions).

## Phase

Phase 3+: AI Architecture Overhaul

## Crates

- `worldwake-ai`

## Dependencies

- S30 (AI Runtime Save/Load Parity) — the invalidation conditions must survive save/load. **COMPLETED.**
- S29 (Planning State Structural Sharing) — reduces the cost of re-searches that do occur after invalidation. **COMPLETED.**

## Design Goals

1. **Precise invalidation**: Each exhausted goal's cache entry specifies which world conditions would make it worth re-searching. The cache clears for a goal only when those conditions change.
2. **No over-invalidation**: Bread consumption does not clear the Apple acquisition cache. Position changes at waypoints do not clear travel-irrelevant caches.
3. **No under-invalidation**: Dirtiness increase clears the Wash exhaustion cache when dirtiness crosses the relevant threshold. Resource regeneration at a known source clears acquisition caches when the agent arrives and perceives it.
4. **Profile-driven, not magic numbers**: Invalidation conditions are derived from the goal's `GoalKind` and the agent's belief state, not from hardcoded rules (Principle 2).
5. **Determinism preserved**: Invalidation is a pure function of world state changes, not of timing or order of evaluation.
6. **No locality violations**: Conditions reference only state the agent can observe through the existing `GoalBeliefView`/`RuntimeBeliefView` interface (Principles 12/13). No remote-state polling.

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
    /// Wound state changes (new wound, wound healed, severity change).
    WoundsChanged,
    /// Facility access changes at the agent's current place.
    FacilitiesChanged,
    /// A blocked intent expires (blocker cleanup).
    BlockerExpired,
    /// The set of visible hostiles or hostile targets changes.
    HostilesChanged,
    /// A specific need crosses a threshold (for needs-driven goals).
    NeedCrossedThreshold {
        need: HomeostaticNeedId,
        /// The need level at exhaustion time. Re-search when the
        /// absolute difference exceeds this delta.
        threshold_delta: Permille,
    },
    /// The bound target entity is no longer alive.
    /// When this fires, the entry is removed unconditionally —
    /// candidate generation will not produce this goal anyway.
    TargetDead(EntityId),
}
```

Note: uses `HomeostaticNeedId` from `worldwake-core::needs`, not a new `NeedKind`.

### 2. `ExhaustionEntry` struct (extended)

Extend the live `ExhaustionEntry` shape. Remove the `Copy` derive since the new fields contain `Vec`. Preserve `exhausted_at` and `count` for compatibility with the existing backoff logic:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExhaustionEntry {
    /// Tick when the goal last entered the active skip window, if any.
    pub exhausted_at: Option<Tick>,
    /// Consecutive exhaustion count used for exponential backoff.
    /// Resets to 0 when an invalidation condition fires (the world
    /// genuinely changed, so the goal deserves a full-budget re-search).
    pub count: u8,
    /// Conditions that would warrant re-searching this goal.
    #[serde(default)]
    pub invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
    /// Snapshot of relevant state at exhaustion time (for delta detection).
    #[serde(default)]
    pub baseline: ExhaustionBaseline,
}
```

The `#[serde(default)]` on new fields ensures backward compatibility with save files that contain the old 2-field `ExhaustionEntry` (format version 6). No `SAVE_FORMAT_VERSION` bump is required because the existing post-load validation in `agent_tick/mod.rs:143-173` already prunes stale entries, and entries without conditions will be treated as always-invalidated (conservative fallback per Invariant 5).

### 3. `ExhaustionBaseline` struct

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExhaustionBaseline {
    /// Agent's effective place at exhaustion time.
    pub position: Option<EntityId>,
    /// Agent's homeostatic needs at exhaustion time.
    pub needs: Option<HomeostaticNeeds>,
    /// Commodity quantities relevant to this goal at exhaustion time.
    pub commodity_quantities: Vec<(CommodityKind, Quantity)>,
    /// Unique item counts relevant to this goal at exhaustion time.
    pub unique_item_counts: Vec<(UniqueItemKind, u32)>,
    /// Wound count at exhaustion time.
    pub wound_count: usize,
    /// Number of visible hostiles at exhaustion time.
    pub hostile_count: usize,
}
```

### 4. `derive_invalidation_conditions` function

Maps each `GoalKind` variant to the set of conditions that would make the goal's search space materially different. Returns the conditions AND the baseline snapshot:

```rust
fn derive_invalidation_conditions(
    goal: &GoalKind,
    agent: EntityId,
    view: &dyn GoalBeliefView,
    recipe_registry: &RecipeRegistry,
) -> (Vec<ExhaustionInvalidationCondition>, ExhaustionBaseline)
```

#### Full GoalKind condition mapping table

| GoalKind | Conditions | Rationale |
|----------|-----------|-----------|
| `ConsumeOwnedCommodity { commodity }` | `CommodityChanged(commodity)` | Re-search when the agent's quantity of that commodity changes |
| `AcquireCommodity { commodity, purpose: SelfConsume }` | `PositionChanged`, `CommodityChanged(commodity)` | New place = new sources/sellers; quantity change = already got some |
| `AcquireCommodity { commodity, purpose: Restock }` | `PositionChanged`, `CommodityChanged(commodity)`, `CommodityChanged(Coin)` | Restock also requires payment |
| `AcquireCommodity { commodity, purpose: RecipeInput(recipe_id) }` | `PositionChanged`, `CommodityChanged(commodity)` | New place may have new sources |
| `Sleep` | `NeedCrossedThreshold { Fatigue, delta: Permille(100) }`, `FacilitiesChanged` | Re-search when fatigue changes significantly or sleep facilities change |
| `Relieve` | `NeedCrossedThreshold { Bladder, delta: Permille(100) }`, `PositionChanged` | Bladder pressure change; relieve-location access may change |
| `Wash` | `NeedCrossedThreshold { Dirtiness, delta: Permille(100) }`, `FacilitiesChanged` | Dirtiness threshold; wash facility access |
| `EngageHostile { target }` | `PositionChanged`, `WoundsChanged`, `TargetDead(target)` | Position for reach, wounds for combat readiness, target death ends goal |
| `ReduceDanger` | `PositionChanged`, `WoundsChanged`, `HostilesChanged` | Danger is location+wound+hostile dependent |
| `TreatWounds { patient }` | `PositionChanged`, `WoundsChanged`, `CommodityChanged(Medicine)`, `TargetDead(patient)` | Medicine availability, wound state, patient death |
| `ProduceCommodity { recipe_id }` | `PositionChanged`, `FacilitiesChanged`, + `CommodityChanged(input)` for each recipe input | Position for workstation access, facilities, input availability |
| `SellCommodity { commodity }` | `PositionChanged`, `CommodityChanged(commodity)` | Need to be at a market with the commodity |
| `RestockCommodity { commodity }` | `PositionChanged`, `CommodityChanged(commodity)`, `CommodityChanged(Coin)` | Position for sources, commodity and coin for trade |
| `MoveCargo { commodity, destination }` | `PositionChanged`, `CommodityChanged(commodity)` | Position for pickup, commodity availability |
| `LootCorpse { corpse }` | `PositionChanged`, `TargetDead(corpse)` | Position for reach; if corpse entity is purged, stop trying |
| `BuryCorpse { corpse, burial_site }` | `PositionChanged`, `TargetDead(corpse)` | Position for reach; corpse purge |
| `ShareBelief { listener, .. }` | `PositionChanged`, `TargetDead(listener)` | Different listeners at different places; listener death |
| `ClaimOffice { office }` | `PositionChanged`, `BlockerExpired` | Position for office access; blocker expiry for retry |
| `SupportCandidateForOffice { office, candidate }` | `PositionChanged`, `BlockerExpired`, `TargetDead(candidate)` | Position, blocker expiry, candidate death |
| `InvestigateViolation { violation_id, place }` | `PositionChanged` | Agent needs to reach the violation place |
| `StealItem { target_item }` | `PositionChanged`, `TargetDead(target_item)` | Position for reach; item destruction |
| `Accuse { crime_register, accused, .. }` | `PositionChanged`, `TargetDead(accused)` | Position for institutional access; accused death |
| `PunishAccused { office, accused, .. }` | `PositionChanged`, `TargetDead(accused)`, `BlockerExpired` | Position for office; accused death; blocker expiry |

**Notes on the table:**
- `TargetDead` uses `GoalBeliefView::is_alive()` on the bound entity. For item entities (LootCorpse corpse, StealItem target_item), this covers entity purge.
- `ProduceCommodity` derives recipe inputs from `RecipeRegistry::get(recipe_id)` to populate `CommodityChanged` conditions for each input commodity.
- `NeedCrossedThreshold` uses `Permille(100)` as the default delta (10% of the 0-1000 range). This is not a magic number — it represents a meaningful change in the need level that could alter the search space (e.g., moving from below a ThresholdBand boundary to above it). The delta could be derived from the agent's `DriveThresholds` if finer tuning is needed, but the fixed 100 is a reasonable starting point.
- `HostilesChanged` fires when the count of `visible_hostiles_for(agent)` changes. Used only for `ReduceDanger` which is not target-bound.

### 5. Replace `reset_exhausted_goals_if_needed` with `invalidate_exhausted_goals`

Instead of checking a single dirty-bit mask and clearing ALL goals, iterate the exhaustion map and check each goal's specific conditions against the current belief state:

```rust
fn invalidate_exhausted_goals(
    exhaustion_cache: &mut BTreeMap<GoalKey, ExhaustionEntry>,
    view: &dyn GoalBeliefView,
    agent: EntityId,
    currently_in_transit: bool,
) {
    exhaustion_cache.retain(|_goal, entry| {
        if entry.invalidation_conditions.is_empty() {
            // Conservative fallback: entries without conditions
            // (e.g., from old save files) are always invalidated.
            return false;
        }
        // Keep the entry (don't invalidate) if none of the conditions changed.
        !entry.invalidation_conditions.iter().any(|condition| {
            condition_changed(condition, &entry.baseline, view, agent, currently_in_transit)
        })
    });
}
```

**Invalidation semantics**: `.retain()` removes the entire entry when conditions fire. This resets `count` to 0 on the next exhaustion, giving the goal a full-budget re-search since the world genuinely changed. This is intentional — the old `count` reflected a stale world state.

**Position filtering**: The `currently_in_transit` parameter preserves the existing behavior from `reset_exhausted_goals_if_needed`: `PositionChanged` only fires when the agent has genuinely arrived at a new place (transition from in-transit to settled, or was already settled), not at every waypoint during multi-leg travel.

### 6. `condition_changed` function

```rust
fn condition_changed(
    condition: &ExhaustionInvalidationCondition,
    baseline: &ExhaustionBaseline,
    view: &dyn GoalBeliefView,
    agent: EntityId,
    currently_in_transit: bool,
) -> bool {
    match condition {
        ExhaustionInvalidationCondition::PositionChanged => {
            // Only trigger on genuine arrival, not mid-transit waypoints.
            if currently_in_transit {
                return false;
            }
            view.effective_place(agent) != baseline.position
        }
        ExhaustionInvalidationCondition::CommodityChanged(kind) => {
            let current = view.commodity_quantity(agent, *kind);
            !baseline
                .commodity_quantities
                .iter()
                .any(|(k, q)| k == kind && *q == current)
                // If the commodity wasn't in the baseline, any non-zero quantity is a change.
                || (!baseline.commodity_quantities.iter().any(|(k, _)| k == kind)
                    && current > Quantity(0))
        }
        ExhaustionInvalidationCondition::UniqueItemChanged(kind) => {
            let current = view.unique_item_count(agent, *kind);
            !baseline
                .unique_item_counts
                .iter()
                .any(|(k, c)| k == kind && *c == current)
        }
        ExhaustionInvalidationCondition::WoundsChanged => {
            view.wounds(agent).len() != baseline.wound_count
        }
        ExhaustionInvalidationCondition::FacilitiesChanged => {
            // Delegate to dirty-bit: if FACILITIES was set since last check,
            // the facility access at the agent's place changed. This avoids
            // duplicating the facility-signature comparison logic.
            // Implementation note: the caller can pass this as a pre-computed
            // boolean derived from the dirty set.
            //
            // For simplicity, compare against baseline: true if facility access
            // at the agent's effective place differs from exhaustion time. The
            // exact comparison mechanism can reuse the existing
            // last_facility_access_signature pattern.
            true // Conservative: always re-search if FacilitiesChanged is in conditions
                 // Refined in implementation via dirty-bit delegation.
        }
        ExhaustionInvalidationCondition::BlockerExpired => {
            // Delegate to dirty-bit: BLOCKER_CLEANUP was set.
            // Same delegation pattern as FacilitiesChanged.
            true // Conservative default; refined in implementation.
        }
        ExhaustionInvalidationCondition::HostilesChanged => {
            view.visible_hostiles_for(agent).len() != baseline.hostile_count
        }
        ExhaustionInvalidationCondition::NeedCrossedThreshold {
            need,
            threshold_delta,
        } => {
            if let Some(current_needs) = view.homeostatic_needs(agent) {
                if let Some(baseline_needs) = &baseline.needs {
                    let current_val = need_value(&current_needs, *need);
                    let baseline_val = need_value(baseline_needs, *need);
                    let diff = if current_val.0 > baseline_val.0 {
                        current_val.0 - baseline_val.0
                    } else {
                        baseline_val.0 - current_val.0
                    };
                    diff >= threshold_delta.0
                } else {
                    true // No baseline needs = conservative invalidation
                }
            } else {
                false // No current needs = can't evaluate, keep cached
            }
        }
        ExhaustionInvalidationCondition::TargetDead(target) => !view.is_alive(*target),
    }
}

fn need_value(needs: &HomeostaticNeeds, need: HomeostaticNeedId) -> Permille {
    match need {
        HomeostaticNeedId::Hunger => needs.hunger,
        HomeostaticNeedId::Thirst => needs.thirst,
        HomeostaticNeedId::Fatigue => needs.fatigue,
        HomeostaticNeedId::Bladder => needs.bladder,
        HomeostaticNeedId::Dirtiness => needs.dirtiness,
    }
}
```

### 7. Remove EXHAUSTION_SKIP_TTL

With precise invalidation, the TTL-based periodic re-search is no longer needed. Goals stay cached until their specific conditions change. Remove:

- `const EXHAUSTION_SKIP_TTL: u64 = 20` from `planning.rs`
- `exhaustion_skip_active()` function from `planning.rs`
- The tick-based filtering in `build_candidate_plans` (the `exhaustion_cache.get(&c.grounded.key).is_some_and(|entry| exhaustion_skip_active(entry, current_tick))` filter)

Replace with a simpler check: skip any goal that has `exhausted_at.is_some()` (the goal has been exhausted and its conditions have not yet fired).

### 8. Update `record_exhausted_goals` to capture conditions

When a goal exhausts the search budget, derive its invalidation conditions and baseline, then store them in the exhaustion entry:

```rust
fn record_exhausted_goals(
    exhaustion_cache: &mut BTreeMap<GoalKey, ExhaustionEntry>,
    plans: &[(GoalKey, PlanSearchResult, ...)],
    tick: Tick,
    agent: EntityId,
    view: &dyn GoalBeliefView,
    recipe_registry: &RecipeRegistry,
) {
    for (key, result, _, _) in plans {
        if matches!(
            result,
            PlanSearchResult::BudgetExhausted { .. }
                | PlanSearchResult::FrontierExhausted { .. }
        ) {
            let (conditions, baseline) =
                derive_invalidation_conditions(&key.kind, agent, view, recipe_registry);
            exhaustion_cache
                .entry(*key)
                .and_modify(|entry| {
                    entry.exhausted_at = Some(tick);
                    entry.invalidation_conditions = conditions.clone();
                    entry.baseline = baseline.clone();
                })
                .or_insert(ExhaustionEntry {
                    exhausted_at: Some(tick),
                    count: 0,
                    invalidation_conditions: conditions,
                    baseline,
                });
        } else {
            // Goal was searched and did NOT exhaust — clear entry entirely.
            exhaustion_cache.remove(key);
        }
    }
}
```

### 9. Update exponential backoff in `build_candidate_plans`

The existing budget-halving logic (`entry.count.min(3)` shift) is preserved but now uses `exhausted_at.is_some()` as the skip predicate instead of the TTL check:

```rust
// In build_candidate_plans:
let candidates_to_plan: Vec<_> = ranked_candidates
    .iter()
    .filter(|c| {
        !exhaustion_cache
            .get(&c.grounded.key)
            .is_some_and(|entry| entry.exhausted_at.is_some())
    })
    .take(usize::from(budget.max_candidates_to_plan))
    .collect();
```

The budget reduction for goals with prior exhaustion history remains unchanged (it reads `entry.count` which now resets on invalidation).

## Component Registration

No new ECS components. `ExhaustionEntry`, `ExhaustionInvalidationCondition`, and `ExhaustionBaseline` are AI-layer runtime state, not world state. They are serialized within the AI runtime payload (S30 boundary), not as ECS components.

## Save/Load Compatibility

The new `ExhaustionEntry` fields use `#[serde(default)]` to maintain backward compatibility with existing save files (format version 6). Old entries without conditions will have empty `invalidation_conditions`, triggering the conservative always-invalidate fallback (Invariant 5). No `SAVE_FORMAT_VERSION` bump is required.

Post-load validation in `agent_tick/mod.rs:143-173` already prunes entries with dead entity/place references. S31 adds no new entity references that require additional pruning (the `TargetDead` condition references the same entities already tracked by `GoalKey`).

## SystemFn Integration

### `worldwake-ai`

- Replace `reset_exhausted_goals_if_needed` in `planning.rs` with `invalidate_exhausted_goals`.
- Extend `ExhaustionEntry` in `decision_runtime.rs` with `invalidation_conditions` and `baseline` fields; remove `Copy` derive.
- Add `ExhaustionInvalidationCondition`, `ExhaustionBaseline`, `derive_invalidation_conditions`, `invalidate_exhausted_goals`, `condition_changed`, and `need_value` in a new `exhaustion.rs` module within `crates/worldwake-ai/src/`.
- Remove `EXHAUSTION_SKIP_TTL` constant and `exhaustion_skip_active()` from `planning.rs`.
- Update `record_exhausted_goals` to accept `agent`, `view`, and `recipe_registry` parameters and capture conditions+baseline.
- Update the skip predicate in `build_candidate_plans` from TTL-check to `exhausted_at.is_some()`.

## Cross-System Interactions (Principle 24)

No cross-system interactions. The invalidation conditions read from the existing `GoalBeliefView` interface (which already abstracts over world state and agent beliefs). No system calls another system's logic. The conditions are derived from the goal's `GoalKind` semantics, not from querying other systems directly.

## FND-01 Section H

### H.1 Information-Path Analysis

Exhaustion invalidation reads the same belief state that candidate generation already reads. No new information paths are introduced. The conditions (`PositionChanged`, `CommodityChanged`, etc.) are derived from comparing the current belief snapshot against the baseline captured at exhaustion time — both obtained through the existing `GoalBeliefView`.

### H.2 Positive-Feedback Analysis

**No amplifying loops introduced.** The invalidation system is purely reactive: world state changes -> condition check -> cache clear or retain. It does not generate actions, modify world state, or influence other agents.

Potential concern: could precise invalidation cause a goal to oscillate between "exhausted" and "not exhausted"? Yes, but this is correct behavior — the world changed, so re-searching is appropriate. The cost is bounded by the next point.

### H.3 Concrete Dampeners

Two dampeners limit computational cost:

1. **Condition-gated invalidation**: Unlike TTL-based re-search (which re-searches everything every N ticks), condition-based invalidation only re-searches when the specific relevant world state changed. A goal that was unsolvable because no apples exist will only re-enter search when the agent's apple quantity changes or the agent moves — not when unrelated bread is consumed. This eliminates the dominant cost driver (52 redundant re-searches per scenario).

2. **Exponential backoff on re-exhaustion**: The `count`-based budget halving (512 -> 256 -> 128 -> 64) is preserved. Although `count` resets to 0 when conditions fire (giving one full-budget re-search for a genuinely changed world), if the goal exhausts again in the new world state, `count` starts climbing again. The floor of 64 expansions limits the worst case to 12.5% of full budget.

Together, these mechanisms ensure that the system is no more expensive than the current TTL approach in the worst case, and significantly cheaper in the common case.

### H.4 Stored vs Derived State

- **Stored (in AI runtime, serialized with S30)**: `ExhaustionEntry` (invalidation conditions + baseline snapshot + exhaustion tick + backoff count).
- **Derived (not stored)**: The invalidation CHECK (comparison of current state against baseline) is a transient computation performed at the start of each planning tick. The result is a boolean (invalidated or not) that is never stored.
- **Source of truth**: The exhaustion conditions are derived from `GoalKind` semantics (immutable constants) and the agent's belief state at exhaustion time (captured in the baseline). Neither is an abstract score — both are concrete references to world state (Principle 3).

## Invariants

1. A goal is invalidated (removed from cache, re-searched with full budget) if and only if at least one of its recorded conditions has changed since the baseline was captured.
2. `derive_invalidation_conditions` is a pure function of `GoalKind` + agent belief state. Given the same inputs, it produces the same conditions.
3. The exhaustion cache is deterministic: same world state history -> same cache contents.
4. No goal is permanently cached — every condition set includes at least one condition that references mutable world state (position, commodity quantities, needs, wounds, hostiles, or target liveness). World evolution will eventually trigger one of these conditions.
5. The system degrades gracefully to "re-search everything" when `invalidation_conditions` is empty (conservative fallback for entries from old save files or unknown goal kinds).

## Tests

- [ ] Unit test: `derive_invalidation_conditions` for each of the 23 GoalKind variants returns non-empty conditions.
- [ ] Unit test: `condition_changed` correctly detects position change (including in-transit filtering).
- [ ] Unit test: `condition_changed` correctly detects commodity quantity change.
- [ ] Unit test: `condition_changed` correctly detects wound count change.
- [ ] Unit test: `condition_changed` correctly detects need threshold crossing.
- [ ] Unit test: `condition_changed` correctly detects hostile count change.
- [ ] Unit test: `condition_changed` correctly detects target death.
- [ ] Unit test: entries with empty conditions are always invalidated (backward compatibility).
- [ ] Golden test: Agent with exhausted `AcquireCommodity(Apple)` does NOT re-search when bread is consumed (no over-invalidation).
- [ ] Golden test: Agent with exhausted `Wash` goal re-searches when dirtiness crosses threshold (no under-invalidation).
- [ ] Golden test: `golden_save_load_round_trip_under_ai` passes without driver reset (S30 parity preserved).
- [ ] Golden test: `golden_wash_action` passes (the test that broke in exp-005 due to indefinite caching).
- [ ] Golden test: `golden_three_way_need_competition` passes (the test that broke in exp-005).
- [ ] Golden test: Old save files (without invalidation conditions) load cleanly and behave correctly.
- [ ] Profiling: fewer total exhausted re-searches than TTL=20 approach on `golden_world_runs_without_observers`.
- [ ] All workspace tests pass (`cargo test --workspace`).

## Acceptance Criteria

1. EXHAUSTION_SKIP_TTL is removed entirely — invalidation is condition-based, not time-based.
2. No over-invalidation: irrelevant commodity changes do not trigger re-searches.
3. No under-invalidation: needs-driven goals re-search when the relevant need crosses a threshold.
4. All golden tests pass (including the 4 that broke in exp-005 with indefinite caching).
5. Profiling shows fewer total exhausted searches than TTL=20 on the `golden_world_runs_without_observers` test.
6. Per-goal invalidation conditions are documented and testable for all 23 GoalKind variants.
7. Save/load backward compatibility: old save files without conditions load cleanly.
8. `ExhaustionEntry` no longer derives `Copy`.

## References

- golden-perf campaign: exp-005 (indefinite caching broke 4 tests), exp-013/014/015 (TTL tuning), exp-016 (TTL=32 too aggressive)
- `crates/worldwake-ai/src/agent_tick/planning.rs` — `reset_exhausted_goals_if_needed`, `record_exhausted_goals`, `EXHAUSTION_SKIP_TTL`
- `crates/worldwake-ai/src/decision_runtime.rs` — `exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>`
- `crates/worldwake-ai/src/goal_model.rs` — `GoalKind` (23 variants) and `GoalKindPlannerExt`
- `crates/worldwake-core/src/needs.rs` — `HomeostaticNeedId`
- `docs/FOUNDATIONS.md` Principles 2, 3, 10, 11, 12, 13, 18, 19, 24, 25
