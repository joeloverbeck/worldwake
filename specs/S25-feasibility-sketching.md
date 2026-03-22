**Status**: PENDING

# S25: Feasibility Sketching

## Summary

Add a cheap feasibility pre-check to the candidate ranking pipeline that estimates whether a goal is locally actionable before committing full GOAP search budget. Goals with low feasibility are demoted within their priority class, not excluded — preserving the principle that goals are desired world conditions, not privileged solutions, while preventing wasted search effort on provably unreachable goals.

## Phase

Phase 3+: AI Architecture Overhaul (Step 13.5, Wave 2)

## Crate

`worldwake-ai`

## Dependencies

- S20 (AI pipeline structural cleanup — cleaner module boundaries simplify insertion of the feasibility stage)

## FOUNDATIONS Alignment

- **P18** (Resource-Bounded Practical Reasoning Over Scripts): Agents should reason tractably. Wasting all 4 planning slots on infeasible high-motive goals while obvious actions go unplanned is not bounded reasoning — it is a planner architecture weakness. Feasibility sketching is a bounded-rational heuristic: spend cheap computation to avoid expensive dead ends.
- **P20** (Agent Diversity Through Concrete Variation): Different agents may have different feasibility landscapes. An agent who knows a route sees `Uncertain`; one who does not sees `Unlikely`. The sketch respects per-agent beliefs, not global truth.
- **P12** (World State Is Not Belief State): Feasibility checks use only the agent's `GoalBeliefView`, never authoritative world state. An agent with false beliefs about a route gets `Likely` or `Uncertain` based on what they believe, not what is true.

## Motivation

The current pipeline generates candidates, ranks them by `(GoalPriorityClass, motive_score)`, and searches the top `max_candidates_to_plan` (default 4) with full GOAP. If the highest-motive goal requires traveling to an unknown location, or the target is known-dead, or blocker memory says "this is blocked for N more ticks," the search expands many nodes finding nothing while a directly-actionable lower-motive goal sits unsearched.

**Example**: Agent has critical hunger (motive 900) for food at a place they cannot reach (no adjacent path known) AND critical hunger (motive 600) for food at their current location. Currently, the unreachable goal takes planning slot 1 and wastes the GOAP search budget. With feasibility sketching, the local food goal is searched first because it is `Likely` while the unreachable one is `Unlikely` — both within the same `Critical` priority class.

**Non-goal**: This spec does not exclude any goal from search. An `Unlikely` goal is still searched if budget permits. The only change is ordering within priority classes.

## Design

### FeasibilityHint Enum

```rust
/// Cheap pre-GOAP estimate of whether a goal is locally actionable.
/// Used to reorder candidates within the same `GoalPriorityClass` —
/// never to exclude goals from search.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum FeasibilityHint {
    /// Direct affordance exists at current location, or one-step plan is obvious.
    Likely,
    /// Cannot determine feasibility cheaply — needs full GOAP search.
    Uncertain,
    /// Blocker memory or missing prerequisites strongly suggest infeasibility.
    Unlikely,
}
```

The `Ord` derivation gives `Likely < Uncertain < Unlikely` (enum variant order). The sorting comparator will reverse this so `Likely` sorts first.

### Feasibility Sketch Function

```rust
/// Derive a cheap feasibility estimate for a ranked goal using only the
/// agent's beliefs and blocker memory. Never touches authoritative world state.
pub fn feasibility_hint(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    goal: &RankedGoal,
    blocked_memory: &BlockedIntentMemory,
    current_tick: Tick,
) -> FeasibilityHint
```

The function takes `GoalBeliefView` (the same trait used by `rank_candidates`), not `RuntimeBeliefView` or `Topology`. This ensures it operates strictly within the agent's belief boundary.

**Checks (in order, short-circuit on first conclusive result):**

1. **Non-generation blocker check**: Is there an active non-expired blocker for this goal key where `blocks_goal_generation()` returns false (i.e., soft blockers like `ExclusiveFacilityUnavailable` or `SourceDepleted` that do not suppress candidate generation but indicate likely infeasibility)? -> `Unlikely`. Note: hard blockers that return `blocks_goal_generation() == true` already suppress the goal during candidate generation, so they will never appear here.

2. **Active hard blocker present but not yet expired**: Check `blocked_memory.intents` for any entry matching the goal key with `expires_tick > current_tick`, regardless of `blocks_goal_generation()`. If found -> `Unlikely`. This catches blockers like `NoKnownPath`, `NoKnownSeller`, `TargetGone` that have not yet expired but whose goal was re-generated (e.g., by a different evidence entity).

3. **Target at current location**: If `goal.grounded.evidence_places` contains the agent's current `effective_place` -> `Likely`. The goal has evidence at the agent's location, suggesting a local affordance.

4. **Possessed commodity for consume/relieve goals**: For `GoalKind::Consume` or `GoalKind::Relieve` or `GoalKind::Wash` goals with a known commodity, if the agent already possesses that commodity (`view.commodity_quantity(agent, commodity) > Quantity::ZERO`) -> `Likely`.

5. **No evidence places reachable**: If `goal.grounded.evidence_places` is non-empty and none of them are the agent's current location, and for each evidence place, no adjacent path exists from the agent's location (checking `view.adjacent_places_with_travel_ticks(agent_place)` does not include any evidence place as an immediate neighbor) -> `Unlikely`. Note: this is a one-hop adjacency check, not full pathfinding. It is intentionally conservative — a place two hops away gets `Uncertain`, not `Unlikely`.

6. **Default** -> `Uncertain`.

**Cost**: Each call performs at most one `effective_place` lookup, one `commodity_quantity` lookup, one `adjacent_places_with_travel_ticks` call, and one linear scan of `BlockedIntentMemory.intents`. All are O(1) or O(small) relative to the GOAP search budget of 512 node expansions.

### Integration with Ranking

After `rank_candidates()` produces a `RankingOutcome`, apply `feasibility_hint()` to each `RankedGoal` and re-sort. The new comparator becomes:

```
Within the same GoalPriorityClass:
  1. Likely goals   (sorted by motive_score descending)
  2. Uncertain goals (sorted by motive_score descending)
  3. Unlikely goals  (sorted by motive_score descending)
```

Goals do NOT cross priority class boundaries. A `Critical + Unlikely` goal still outranks a `Low + Likely` goal. Feasibility only reorders within the same priority class.

**Implementation approach**: Add a `feasibility: FeasibilityHint` field to `RankedGoal`. Compute it after `rank_candidates()` returns, then re-sort using an updated `compare_ranked_goals()` that inserts the feasibility comparison between `priority_class` and `motive_score`:

```rust
fn compare_ranked_goals(left: &RankedGoal, right: &RankedGoal) -> Ordering {
    right.priority_class.cmp(&left.priority_class)
        .then_with(|| left.feasibility.cmp(&right.feasibility))  // Likely < Uncertain < Unlikely
        .then_with(|| right.motive_score.cmp(&left.motive_score))
        .then_with(|| /* existing tiebreakers */)
}
```

### Integration Point in agent_tick

The feasibility annotation happens in `read_phase_result()` (agent_tick.rs ~line 832), between `rank_candidates()` and the return of `ReadPhaseResult`. The ranked goals are annotated and re-sorted before being passed to the planning phase that calls `.take(max_candidates_to_plan)`.

### Decision Trace Integration

Add a `feasibility: FeasibilityHint` field to `RankedGoalSummary` in `decision_trace.rs`. The `dump_agent()` output will show feasibility alongside priority class and motive for each candidate, making it visible why ordering changed.

### Budget Allocation (Future Extension, Not in Scope)

A future spec could give `Unlikely` goals a reduced `max_node_expansions` or `beam_width`. This spec only reorders candidates. The budget knob is noted here for design continuity but is explicitly out of scope.

## Tickets

### S25-001: Add FeasibilityHint enum and feasibility_hint() function

- Add `FeasibilityHint` enum to `worldwake-ai` (in a new `feasibility.rs` module or within `goal_model.rs`)
- Implement `feasibility_hint()` with checks 1-6 as specified above
- All checks use `GoalBeliefView`, never authoritative world state
- Add `feasibility: FeasibilityHint` field to `RankedGoal` (default `Uncertain` for backward compat in tests that construct `RankedGoal` directly)
- **Verify**: Focused unit tests with mock `GoalBeliefView` covering each check path (blocker present, target at current location, commodity possessed, no reachable evidence places, default uncertain)
- **Verify**: `cargo test -p worldwake-ai` — existing tests compile and pass (new field initialized to `Uncertain` where needed)

### S25-002: Integrate feasibility into candidate ordering

- After `rank_candidates()` in `read_phase_result()`, compute `feasibility_hint()` for each `RankedGoal` and store in the new field
- Update `compare_ranked_goals()` to include feasibility between priority class and motive score
- Re-sort the ranked list with the updated comparator
- **Verify**: `cargo test -p worldwake-ai` — all golden tests pass. Some may show changed tick counts if agents now find food faster; verify new behavior is strictly better (agent acts more sensibly)

### S25-003: Add feasibility to decision traces

- Add `feasibility: FeasibilityHint` field to `RankedGoalSummary` in `decision_trace.rs`
- Populate during trace construction in agent_tick planning phase
- Update `dump_agent()` format string to include feasibility hint per candidate
- Update `summary()` output to mention feasibility if non-`Uncertain`
- **Verify**: Enable tracing in one golden test, confirm feasibility hints appear in trace output

### S25-004: Golden test verification and documentation

- Run all golden tests: `cargo test -p worldwake-ai --test golden_*`
- For any test where behavior changes (different tick counts, different action sequences), verify the new behavior is an improvement and document the change in a brief comment in the test
- If any test regresses (agent does something worse), investigate — the feasibility check may have a false positive/negative that needs correction
- **Verify**: All golden tests pass, no regressions

### S25-005: Workspace verification

- `cargo test --workspace` — all pass
- `cargo clippy --workspace` — no new warnings
- **Verify**: Clean CI

## FND-01 Section H Analysis

### Information-path analysis

Feasibility sketching introduces no new information paths. It reads the agent's existing `GoalBeliefView` (effective place, commodity quantities, adjacent places) and `BlockedIntentMemory` — both of which are already populated by the perception and failure-handling systems. No agent gains information it would not otherwise have.

### Positive-feedback analysis

None. Feasibility hints are stateless derived computations. They do not create new state, do not feed back into candidate generation, and do not alter blocker memory. A goal demoted by `Unlikely` this tick may be `Likely` next tick if the agent moves or a blocker expires. There are no amplifying loops.

### Concrete dampeners

N/A — no positive-feedback loops to dampen.

### Stored state vs. derived read-model list

- **Stored**: None. `FeasibilityHint` is a transient annotation computed fresh each tick from existing beliefs and blocker memory. It is not persisted in any component or serialized to save files.
- **Derived**: `FeasibilityHint` per `RankedGoal` (derived from `GoalBeliefView` reads + `BlockedIntentMemory` scan).

## Verification

1. `cargo test --workspace` — all pass
2. `cargo clippy --workspace` — no new warnings
3. Decision traces show feasibility hints per candidate when tracing is enabled
4. Agents no longer waste all planning slots on unreachable goals in scenarios where local alternatives exist
5. No goal is permanently excluded — `Unlikely` goals are still searched if budget permits (they appear after `Likely` and `Uncertain` goals within the same priority class)
