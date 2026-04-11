# S90: Mandatory Tactical Scoping

## Summary

Fix three compounding failures in the S88/S89 two-phase planning pipeline that silently bypass tactical scoping, causing the search to devolve to pre-S88 flat A* with 2000-2600 candidates per expansion. The root bug is an evidence guard on the `Explore` tactical goal (`mod.rs:103-105`) that blocks tactical scoping when the candidate generator has populated `evidence_places` — which it does via a broader belief query than the strategic planner uses. Without a tactical goal, the candidate filter returns immediately, travel pruning is a no-op, and landmark extraction has no facts. The search explodes.

This spec introduces four changes: (1) remove the evidence guard and replace it with evidence-directed exploration, (2) fail-fast when the strategic planner produces steps but no tactical goal can be constructed, (3) add a per-agent candidate count safety valve that prevents any search from running with an explosive candidate set, and (4) align the strategic planner's belief query with the candidate generator's so the strategic planner finds the same commodity places and produces proper `SatisfyGoal`/`AcquirePrerequisite` stages instead of falling through to Explore.

**Evidence**: Simulation observer report on `cli-evaluation.ron` (seed 7777, 1440 ticks, post-S89) shows:
- Guard Theron died at tick 422 from hunger — `AcquireCommodity(Water)` budget-exhausted at 224 expansions, 2085 candidates, depth 6
- Kael: `AcquireCommodity(Water)` budget-exhausted at 224 expansions, 2657 candidates, depth 6 at Thornwall Village
- Merchant Vara: `AcquireCommodity(Water)` budget-exhausted at 300 expansions, 2522 candidates, depth 9 at Dusty Trail
- All three agents collapsed to sleep+relieve loops; candidate counts are post-tactical-filter, confirming no tactical filtering applied

**Phase**: 7 (Adjunct — Simulation Remediation)
**Status**: DRAFT
**Crates**: `worldwake-ai`, `worldwake-core`, `worldwake-cli`
**Dependencies**: S88 (completed), S89 (completed)
**Supersedes**: None (fixes defects in S88/S89 pipeline)

## Design Goals

- Eliminate the silent bypass path where `tactical_goal = None` disables all S88/S89 protections
- Ensure every multi-location search either has a tactical goal scoping its candidates or fails fast
- Add a structural safety valve preventing any search from running with an explosive candidate count, regardless of how it got there
- Align the strategic planner's commodity-place discovery with the candidate generator's so the two-phase architecture activates for all reachable commodity sources, not just those with explicit `resource_source` beliefs

## Non-Goals

- Fixing Guard Theron's missing survival goals — goal generation gap for the guard archetype, separate ticket
- Fixing Forager Lina's FreeCarryCapacity 0-step plan trap — planner-execution boundary issue, separate ticket
- Fixing the perception-to-belief pipeline for facility resource sources (why Kael never formed beliefs about the Well at Thornwall Village despite visiting twice) — separate investigation
- Raising `max_node_expansions` as a mitigation
- Modifying landmark extraction or dual frontier algorithms

## FOUNDATIONS Alignment

| Principle | How This Spec Satisfies It |
|-----------|----------------------------|
| FND-20 (Bounded Reasoning) | D2 and D3 enforce that bounded lookahead actually stays bounded. The pre-fix bypass allowed unbounded candidate explosion despite the two-phase architecture being designed to prevent it. D3 adds a structural bound that no future code path can bypass. |
| FND-22 (Agent Diversity) | `max_candidates_per_expansion` is a per-agent field on `CognitiveProfile`, allowing different agents to have different explosion thresholds. |
| FND-12 (Perf Compresses Computation) | D3's safety valve compresses computation without changing what plans are reachable — fail-fast means the plan wasn't findable within budget anyway. D4 aligns belief queries so more plans are findable, expanding reachability. |
| FND-14 (Belief-Only Planning) | All changes operate on the belief surface via `PlanningState`/`PlanningSnapshot`. No omniscient queries introduced. |
| FND-28 (No Backward Compat) | No shims or fallback paths for the old unscoped behavior. The evidence guard is removed, not wrapped. |
| FND-29 (Debuggability) | Decision trace already records `tactical_goal` (from S89). D3 adds a new `BudgetExhausted` reason distinguishing "candidate explosion" from "search space exhaustion." |

## Section H — Causal Hooks Declaration

### H.1 — Motivating consequence gap

With S88/S89 completed, the two-phase architecture is fully implemented but has a silent bypass: when the strategic planner falls back to `Explore` and the candidate generator has populated `evidence_places`, the evidence guard at `mod.rs:103-105` blocks the Explore tactical goal. This produces `tactical_goal = None`, which disables the tactical candidate filter, travel pruning, and landmark extraction. The search runs unscoped with 2000-2600 candidates per expansion — the exact pre-S88 failure mode that S88/S89 were designed to eliminate.

The bypass is not a rare edge case. It activates for every `AcquireCommodity` goal where (a) the candidate generator finds a remote opportunity via `acquisition_path_search_inner` but (b) the strategic planner's narrower `places_with_resource_source` fails to find the same place. This covers the primary survival scenario: agents needing food or water at locations where the resource source requires a facility queue (e.g., Well → `queue_for_facility_use` → drink).

### H.2 — Entities, relations, records introduced

One new field on `CognitiveProfile`: `max_candidates_per_expansion: u16`. Universal profile component (already registered on `EntityKind::Agent`). No new entities or relations.

### H.3 — Actions or world processes that mutate them

None. All changes are planner-internal.

### H.4 — Information produced, travel, observability

Diagnostic only: `BudgetExhausted` result gains a distinction between candidate-explosion and search-exhaustion in debug traces. Not visible in world state.

### H.5 — Conserved quantities

None affected.

### H.6 — Scarce capacities, contention

None introduced.

### H.7 — Partial failures, aftermath

When D2's fail-fast triggers (strategic steps exist but no tactical goal), the result is `FrontierExhausted`. The agent replans on the next tick. This is the same recovery path as any other planning failure.

When D3's safety valve triggers (candidate count exceeds threshold), the result is `BudgetExhausted`. Same recovery path.

Neither failure creates new world state or side effects.

### H.8 — Positive feedback loops amplified

None.

### H.9 — Physical dampeners

N/A.

### H.10 — Cross-system interaction

D4 aligns the strategic planner's `place_supports_commodity()` with the candidate generator's `place_has_direct_acquisition_support()`. Both are in `worldwake-ai`. D3 adds a field to `CognitiveProfile` in `worldwake-core` (existing component, new field only). No cross-crate system interactions introduced.

## Information-path analysis

No information paths introduced or modified. All changes operate on existing belief state from `PlanningState`/`PlanningSnapshot`. The strategic planner's aligned belief query (D4) reads the same belief store it already reads — it just checks more properties of the entities it finds there.

## Positive-feedback analysis

No amplifying loops introduced. The tactical goal is computed once per planning call. The safety valve is a one-shot check per expansion. Neither feeds back into itself.

## Stored state vs. derived read-model list

| Item | Classification | Justification |
|------|---------------|---------------|
| `CognitiveProfile::max_candidates_per_expansion` | Authoritative stored state | Per-agent configurable parameter. Persisted as ECS component. Scenario-definable via `AgentDef`. |
| `TacticalGoal` variants | Transient derived | Computed per planning call from strategic plan. Not stored. |
| Evidence-directed exploration destination | Transient derived | Computed per planning call from `evidence_places`. Not stored. |

## Deliverables

### D1: Remove Evidence Guard, Add Evidence-Directed Exploration

**File**: `crates/worldwake-ai/src/search/mod.rs`

Replace the evidence guard at lines 102-108:

```rust
// BEFORE (broken):
TacticalSubGoal::Explore => {
    (exploration_supports_tactical_barrier(&goal.key.kind)
        && goal.evidence_entities.is_empty()
        && goal.evidence_places.is_empty())
    .then_some(Self::Explore {
        destination: step.destination,
    })
}
```

with:

```rust
// AFTER (fixed):
TacticalSubGoal::Explore => {
    exploration_supports_tactical_barrier(&goal.key.kind).then_some(Self::Explore {
        destination: evidence_directed_destination(goal, step, snapshot),
    })
}
```

Add a helper function `evidence_directed_destination` that:
1. If `goal.evidence_places` is non-empty, selects the nearest evidence place (by `min_perceived_travel_cost_to_any` from the actor's current place). This directs exploration toward known commodity locations rather than random adjacent places.
2. If `goal.evidence_places` is empty, uses the strategic step's default destination (the adjacent-place heuristic from `exploration_plan()`).

This requires passing the `PlanningSnapshot` (or the actor's current place and the snapshot's distance matrix) into `from_strategic_step`. Adjust the function signature accordingly.

Delete the `exploration_supports_tactical_barrier` function. After D1, all `Explore` sub-goals unconditionally produce tactical goals — the function no longer gates anything. If future goal kinds should NOT explore, they should not produce `TacticalSubGoal::Explore` in the strategic planner, not be filtered here.

### D2: Mandatory Tactical Goal for Multi-Location Goals

**File**: `crates/worldwake-ai/src/search/mod.rs`

After the `TacticalGoal::from_strategic_step()` call (line 267-270), add a fail-fast guard:

```rust
let tactical_goal = TacticalGoal::from_strategic_step(
    goal,
    strategic_plan.as_ref().and_then(|plan| plan.steps.first()),
    snapshot,
);

// Fail-fast: if strategic plan requires travel but no tactical goal was produced,
// the search would run unscoped with explosive candidate counts. Return immediately.
if strategic_plan
    .as_ref()
    .is_some_and(|plan| !plan.steps.is_empty())
    && tactical_goal.is_none()
{
    return PlanSearchResult::FrontierExhausted {
        expansions_used: 0,
    };
}
```

This ensures that if a future code change introduces a new `TacticalSubGoal` variant that `from_strategic_step` doesn't handle, the search fails fast rather than running unscoped.

Local goals (strategic plan with empty steps or `None`) are exempt — they run unscoped as before because their candidate counts are bounded by local affordances.

Note: `FrontierExhausted` currently has no fields. If it needs `expansions_used`, add it. Otherwise use the existing variant as-is.

### D3: Candidate Count Safety Valve

**Files**: `crates/worldwake-ai/src/search/mod.rs`, `crates/worldwake-core/src/cognitive_profile.rs`, `crates/worldwake-cli/src/scenario/types.rs`, `crates/worldwake-cli/src/scenario/mod.rs`

**CognitiveProfile change** (`crates/worldwake-core/src/cognitive_profile.rs`):

Add field:
```rust
/// Maximum candidates per expansion before the search aborts.
/// Prevents degenerate unscoped searches from burning expansion budget
/// on explosive candidate sets that will never produce a viable plan.
pub max_candidates_per_expansion: u16,
```

Default: `200`. This is well above the ~20-50 candidates expected under tactical scoping, and well below the 2000+ seen in the bypass path.

**Scenario contract**: Add `max_candidates_per_expansion` to `CognitiveProfile` in `AgentDef` (universal, with default via `unwrap_or_default()`). `CognitiveProfile` is already in `AgentDef` and `spawn_agent()` — the new field is covered by the existing `Default` impl.

**Search change** (`crates/worldwake-ai/src/search/mod.rs`):

After `apply_tactical_candidate_filter()` (around line 386, where `candidates_generated` is computed), add:

```rust
let candidates_generated = candidates.len() as u16;

if candidates_generated > cognitive.max_candidates_per_expansion {
    if let Some(barrier_plan) = best_barrier {
        return PlanSearchResult::Found(Box::new(barrier_plan));
    }
    return PlanSearchResult::BudgetExhausted {
        expansions_used: expansions,
    };
}
```

This is a fail-fast: the search terminates because the candidate set is too large to be productive. If a progress barrier was already found, return that (same pattern as the existing budget-exhaustion check at line 306-312).

### D4: Align Strategic Planner Belief Query

**File**: `crates/worldwake-ai/src/search/strategic.rs`

Replace `place_supports_commodity()` (lines 294-310) with a function that checks the same sources as the candidate generator's `place_has_direct_acquisition_support()` (`crates/worldwake-ai/src/candidate_generation.rs:4195`):

```rust
fn place_supports_commodity(
    state: &PlanningState<'_>,
    place: EntityId,
    commodity: CommodityKind,
) -> bool {
    // Resource sources with available quantity
    state.entities_at(place).into_iter().any(|entity| {
        state.resource_source(entity).is_some_and(|source| {
            source.commodity == commodity && source.available_quantity > Quantity(0)
        })
    })
    // Merchandise profiles with matching sale kinds
    || state.entities_at(place).into_iter().any(|entity| {
        state.merchandise_profile(entity).is_some_and(|profile| {
            profile.sale_kinds.contains(&commodity)
        })
    })
    // Loose unpossessed commodity lots
    || state.entities_at(place).into_iter().any(|entity| {
        state.item_lot_commodity(entity) == Some(commodity)
            && state.commodity_quantity(entity, commodity) > Quantity(0)
            && state.direct_possessor(entity).is_none()
            && state.direct_container(entity).is_none()
    })
}
```

This is functionally identical to `place_has_direct_acquisition_support` from `candidate_generation.rs`, except it doesn't check `corpse_entities_at` (corpse looting is handled by `LootCorpse` goals, not `AcquireCommodity`). If `place_has_direct_acquisition_support` is refactored to be callable from the strategic module in the future, the duplication can be eliminated. For now, alignment by replication is acceptable because the function is small and self-contained.

The key effect: when the candidate generator finds water at Thornwall Village via `listed_sale_lots_at` or `resource_sources_at`, the strategic planner will now find the same place and produce a proper `SatisfyGoal` strategic step → `TravelToGoal` tactical goal. This eliminates the Explore fallback for cases where evidence exists — the Explore fallback is reserved for true no-evidence situations.

### D5: Tests

**File**: `crates/worldwake-ai/src/search/tests.rs`

New tests:

1. **`search_explore_tactical_goal_produced_despite_nonempty_evidence`** — Create a `GroundedGoal` with non-empty `evidence_places`. Mock a strategic plan returning `TacticalSubGoal::Explore`. Assert that `TacticalGoal::from_strategic_step` returns `Some(Explore { .. })`, not `None`.

2. **`search_fail_fast_when_strategic_steps_but_no_tactical_goal`** — Create a strategic plan with non-empty steps. Mock `from_strategic_step` returning `None`. Assert that `search_plan` returns `FrontierExhausted` immediately (0 expansions).

3. **`search_candidate_safety_valve_triggers_at_threshold`** — Set `max_candidates_per_expansion` to a low value (e.g., 5). Create a scenario with more candidates than the threshold. Assert `BudgetExhausted` is returned.

4. **`strategic_place_supports_commodity_finds_sale_lots`** — Unit test for the aligned `place_supports_commodity`. Create a place with a sale lot for Water. Assert the function returns true.

5. **`strategic_place_supports_commodity_finds_loose_items`** — Create a place with an unpossessed Water item lot. Assert true.

6. **`search_evidence_directed_exploration_prefers_evidence_place`** — Create a goal with `evidence_places = {place_B}`. Strategic planner's default exploration picks adjacent place_C. Assert the tactical goal's destination is place_B (the evidence place), not place_C.

Existing S88/S89 tests must continue to pass unchanged.

## Behavioral Guarantees

### Local goals are unaffected

Goals with empty `goal_relevant_places()` (Sleep, Relieve, Wash, ReduceDanger, FreeCarryCapacity) produce no strategic stages. The strategic planner returns `None` or an empty-steps plan. D2's fail-fast only triggers when steps are non-empty. D3's safety valve only triggers on high candidate counts, which local goals don't produce. Local goals continue to run unscoped as before.

### Multi-location goals gain mandatory tactical scoping

Any goal whose strategic plan has non-empty steps will either (a) receive a tactical goal from `from_strategic_step`, or (b) fail fast via D2. No multi-location search can run unscoped.

### The safety valve is a structural bound

D3's `max_candidates_per_expansion` applies regardless of how the search was configured. Even if a future code change introduces a new bypass path, the safety valve catches it. This is a defense-in-depth measure.

### Evidence-directed exploration improves search quality

When the candidate generator found evidence at a remote place but the strategic planner (pre-D4) couldn't find it, the old behavior was: Explore randomly → waste ticks → eventually budget-exhaust. The new behavior (D1) is: Explore toward the evidence place → arrive → perceive local entities → replan with updated beliefs that include the resource source → strategic planner finds the place → TravelToGoal tactical scoping → find plan. After D4, the strategic planner finds the place directly, skipping the exploration step entirely.

## Verification

1. `cargo test -p worldwake-ai` — all existing and new tests pass
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. Re-run observer: `cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 1440`
4. Verify: no budget-exhausted entries with 2000+ candidates
5. Verify: agents at Dusty Trail either find water plans via TravelToGoal tactical scoping, or get evidence-directed Explore tactical goals that direct them toward known water sources
6. Verify: Forager Lina's behavior is unchanged (her issues are out of scope — FreeCarryCapacity trap)
7. Verify: Guard Theron still dies (his issues are out of scope — missing survival goals in goal generation)
