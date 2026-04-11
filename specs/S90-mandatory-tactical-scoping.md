# S90: Mandatory Tactical Scoping

## Summary

Fix three compounding failures in the S88/S89 two-phase planning pipeline that silently bypass tactical scoping, causing the search to devolve to pre-S88 flat A* with 2000-2600 candidates per expansion. The root bug is an evidence guard on the `Explore` tactical goal (`mod.rs:102-108`) that blocks tactical scoping when the candidate generator has populated `evidence_places` — which it does via a broader belief query than the strategic planner uses. Without a tactical goal, the candidate filter returns immediately, travel pruning is a no-op, and landmark extraction has no facts. The search explodes.

This spec introduces three changes: (1) remove the evidence guard and replace it with evidence-directed exploration, (2) fail-fast when the strategic planner produces steps but no tactical goal can be constructed, and (3) add a per-agent candidate count safety valve that prevents any search from running with an explosive candidate set.

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

## Non-Goals

- Fixing Guard Theron's missing survival goals — goal generation gap for the guard archetype, separate ticket
- Fixing Forager Lina's FreeCarryCapacity 0-step plan trap — planner-execution boundary issue, separate ticket
- Fixing the perception-to-belief pipeline for facility resource sources (why Kael never formed beliefs about the Well at Thornwall Village despite visiting twice) — separate investigation
- Raising `max_node_expansions` as a mitigation
- Modifying landmark extraction or dual frontier algorithms
- Aligning the strategic planner's commodity-place discovery with the candidate generator's — needs runtime traces to diagnose the actual divergence point between `goal_relevant_places` (via `places_with_resource_source` + `places_with_sellers` in `goal_model.rs`) and the candidate generator's `place_has_direct_acquisition_support`. Deferred to a follow-up investigation ticket.

## FOUNDATIONS Alignment

| Principle | How This Spec Satisfies It |
|-----------|----------------------------|
| FND-20 (Bounded Reasoning) | D2 and D3 enforce that bounded lookahead actually stays bounded. The pre-fix bypass allowed unbounded candidate explosion despite the two-phase architecture being designed to prevent it. D3 adds a structural bound that no future code path can bypass. |
| FND-22 (Agent Diversity) | `max_candidates_per_expansion` is a per-agent field on `CognitiveProfile`, allowing different agents to have different explosion thresholds. |
| FND-12 (Perf Compresses Computation) | D3's safety valve compresses computation without changing what plans are reachable — fail-fast means the plan wasn't findable within budget anyway. |
| FND-14 (Belief-Only Planning) | All changes operate on the belief surface via `PlanningState`/`PlanningSnapshot`. No omniscient queries introduced. |
| FND-28 (No Backward Compat) | No shims or fallback paths for the old unscoped behavior. The evidence guard is removed, not wrapped. |
| FND-29 (Debuggability) | Decision trace already records `tactical_goal` (from S89). D3's early termination is distinguishable by trace context (candidate count at abort time vs. expansion budget at exhaustion time), not by a new variant. |

## Section H — Causal Hooks Declaration

### H.1 — Motivating consequence gap

With S88/S89 completed, the two-phase architecture is fully implemented but has a silent bypass: when the strategic planner falls back to `Explore` and the candidate generator has populated `evidence_places`, the evidence guard at `mod.rs:102-108` blocks the Explore tactical goal. This produces `tactical_goal = None`, which disables the tactical candidate filter, travel pruning, and landmark extraction. The search runs unscoped with 2000-2600 candidates per expansion — the exact pre-S88 failure mode that S88/S89 were designed to eliminate.

The bypass is not a rare edge case. It activates for every `AcquireCommodity` goal where (a) the candidate generator finds a remote opportunity via `acquisition_path_search_inner` but (b) the strategic planner's `goal_relevant_places` (via `places_with_resource_source` + `places_with_sellers` in `goal_model.rs`) fails to find the same place. This covers the primary survival scenario: agents needing food or water at locations where the resource source requires a facility queue (e.g., Well → `queue_for_facility_use` → drink).

### H.2 — Entities, relations, records introduced

One new field on `CognitiveProfile`: `max_candidates_per_expansion: u16`. Universal profile component (already registered on `EntityKind::Agent`). No new entities or relations.

### H.3 — Actions or world processes that mutate them

None. All changes are planner-internal.

### H.4 — Information produced, travel, observability

Diagnostic only: `BudgetExhausted` result's trace context distinguishes candidate-explosion (D3 triggers at high candidate count) from search-exhaustion (existing budget check triggers at max expansions). Not visible in world state.

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

D3 adds a field to `CognitiveProfile` in `worldwake-core` (existing component, new field only). No cross-crate system interactions introduced.

## Information-path analysis

No information paths introduced or modified. All changes operate on existing belief state from `PlanningState`/`PlanningSnapshot`. The strategic planner reads the same belief store it already reads.

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

Update the function signature to pass the snapshot:

```rust
fn from_strategic_step(
    goal: &GroundedGoal,
    step: Option<&strategic::StrategicStep>,
    snapshot: &PlanningSnapshot,
) -> Option<Self>
```

Update the call site at line 267-270 to pass `snapshot`.

Add a helper function `evidence_directed_destination` that:
1. If `goal.evidence_places` is non-empty, selects the nearest evidence place (by `min_perceived_travel_cost_to_any` from the actor's current place). This directs exploration toward known commodity locations rather than random adjacent places.
2. If `goal.evidence_places` is empty, uses the strategic step's default destination (the adjacent-place heuristic from `exploration_plan()`).

Preserve the `exploration_supports_tactical_barrier` function as the live goal-family boundary. D1 removes only the stale evidence-empty suppression; it does not broaden tactical exploration barriers to every goal family that can currently fall through `strategic::exploration_plan()`.

**Safety note**: The live branch still has goal families beyond `AcquireCommodity` / `SearchForMissing` that can reach `exploration_plan()` without owning a lawful exploration barrier target. D1 should therefore keep the tactical-goal constructor guard and only change how supported exploration chooses its destination when evidence places exist.

### D2: Barrier-Required Explore Classification And Fail-Fast

**Files**: `crates/worldwake-ai/src/search/strategic.rs`, `crates/worldwake-ai/src/search/mod.rs`

First, split the overloaded strategic `Explore` meaning into two explicit variants:
- barrier-required exploration for goal families that lawfully use a tactical explore barrier
- generic fallback exploration for goal families that may continue without a tactical barrier

`strategic::exploration_plan()` should emit the barrier-required variant only for the currently supported families (`AcquireCommodity`, `SearchForMissing`). All other fallback exploration should emit the generic variant.

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
    .and_then(|plan| plan.steps.first())
    .is_some_and(|step| matches!(step.sub_goal, strategic::TacticalSubGoal::<BARRIER_VARIANT>))
    && tactical_goal.is_none()
{
    return PlanSearchResult::FrontierExhausted {
        expansions_used: 0,
    };
}
```

This ensures that barrier-required exploration cannot silently degrade into unscoped search, while lawful generic exploration fallback remains available for other goal families.

### D3: Candidate Count Safety Valve

**Files**: `crates/worldwake-ai/src/search/mod.rs`, `crates/worldwake-core/src/cognitive_profile.rs`, `crates/worldwake-cli/src/scenario/types.rs`, `crates/worldwake-cli/src/scenario/mod.rs`

**CognitiveProfile change** (`crates/worldwake-core/src/cognitive_profile.rs`):

Add field:
```rust
/// Maximum candidates per expansion before the search aborts.
/// Prevents degenerate unscoped searches from burning expansion budget
/// on explosive candidate sets that will never produce a viable plan.
/// Note: `max_candidates_to_plan` limits total candidates across an entire
/// plan search; `max_candidates_per_expansion` limits candidates at a single
/// expansion step.
pub max_candidates_per_expansion: u16,
```

Default: `200`. This is well above the ~20-50 candidates expected under tactical scoping, and well below the 2000+ seen in the bypass path.

**Scenario contract**: Add `max_candidates_per_expansion` to `CognitiveProfile` in `AgentDef` (universal, with default via `unwrap_or_default()`). `CognitiveProfile` is already in `AgentDef` and `spawn_agent()`, but explicit scenario `cognitive_profile` blocks also deserialize the full struct, so the new field must default cleanly for omitted-field RON inputs instead of relying only on runtime `Default`.

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

### D4: Tests

**File**: `crates/worldwake-ai/src/search/tests.rs`

New tests:

1. **`search_explore_tactical_goal_produced_despite_nonempty_evidence`** — Create a `GroundedGoal` with non-empty `evidence_places`. Mock a strategic plan returning `TacticalSubGoal::Explore`. Assert that `TacticalGoal::from_strategic_step` returns `Some(Explore { .. })`, not `None`.

2. **`search_fail_fast_when_strategic_steps_but_no_tactical_goal`** — Create a strategic plan with non-empty steps. Construct a scenario at the `search_plan` level where the tactical goal is `None` (e.g., by introducing a test-only `TacticalSubGoal` variant or by testing the D2 guard directly with a manually-assembled search state where `from_strategic_step` returns `None`). Assert that `search_plan` returns `FrontierExhausted` immediately (0 expansions).

3. **`search_candidate_safety_valve_triggers_at_threshold`** — Set `max_candidates_per_expansion` to a low value (e.g., 5). Create a scenario with more candidates than the threshold. Assert `BudgetExhausted` is returned.

4. **`search_evidence_directed_exploration_prefers_evidence_place`** — Create a goal with `evidence_places = {place_B}`. Strategic planner's default exploration picks adjacent place_C. Assert the tactical goal's destination is place_B (the evidence place), not place_C.

5. **`strategic_explore_only_for_acquisition_and_search`** — Exercise the strategic planner with multiple goal kinds. Assert that only `AcquireCommodity` and `SearchForMissing` produce `TacticalSubGoal::Explore` via the `exploration_plan()` fallback. Confirms `exploration_supports_tactical_barrier` deletion safety.

Existing S88/S89 tests must continue to pass unchanged.

## Behavioral Guarantees

### Local goals are unaffected

Goals with empty `goal_relevant_places()` (Sleep, Relieve, Wash, ReduceDanger, FreeCarryCapacity) produce no strategic stages. The strategic planner returns `None` or an empty-steps plan. D2's fail-fast only triggers when steps are non-empty. D3's safety valve only triggers on high candidate counts, which local goals don't produce. Local goals continue to run unscoped as before.

### Multi-location goals gain mandatory tactical scoping

Any goal whose strategic plan has non-empty steps will either (a) receive a tactical goal from `from_strategic_step`, or (b) fail fast via D2. No multi-location search can run unscoped.

### The safety valve is a structural bound

D3's `max_candidates_per_expansion` applies regardless of how the search was configured. Even if a future code change introduces a new bypass path, the safety valve catches it. This is a defense-in-depth measure.

### Evidence-directed exploration improves search quality

When the candidate generator found evidence at a remote place but the strategic planner couldn't find it, the old behavior was: Explore randomly → waste ticks → eventually budget-exhaust. The new behavior (D1) is: Explore toward the evidence place → arrive → perceive local entities → replan with updated beliefs that include the resource source → strategic planner finds the place → TravelToGoal tactical scoping → find plan.

## Verification

1. `cargo test -p worldwake-ai` — all existing and new tests pass
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. Re-run observer: `cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 1440`
4. Verify: no budget-exhausted entries with 2000+ candidates
5. Verify: agents at Dusty Trail either find water plans via TravelToGoal tactical scoping, or get evidence-directed Explore tactical goals that direct them toward known water sources
6. Verify: Forager Lina's behavior is unchanged (her issues are out of scope — FreeCarryCapacity trap)
7. Verify: Guard Theron still dies (his issues are out of scope — missing survival goals in goal generation)
