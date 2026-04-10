# S86: Planner Pre-Expansion Candidate Heuristics

## Summary

Add a pre-expansion candidate scoring and truncation stage to the GOAP planner's A* search loop. Currently, all search candidates (1400–2600 per expansion) are fed through the expensive `build_successor_detailed` pipeline before the post-expansion beam truncates non-terminal successors to `beam_width` (default 8). This wastes the entire expansion budget evaluating irrelevant candidates at depth 0, preventing multi-location plans (travel + acquire) from ever being found.

The fix introduces a lightweight scoring function that ranks candidates by goal relevance and spatial alignment *before* successor construction, then truncates to a configurable per-agent cap. This lets the budget stretch across depths instead of being consumed by breadth at depth 0.

**Phase**: 7 (Adjunct — Simulation Remediation Phase 3)
**Status**: Draft
**Crates**: `worldwake-ai`, `worldwake-core`
**Dependencies**: None (operates on existing planner infrastructure from E13, S53, S83)

## Design Goals

- Enable the planner to find multi-location plans (travel to location B + acquire resource) within the existing expansion budget (224–300)
- Reduce per-expansion cost by scoring candidates cheaply before the expensive `build_successor_detailed` call
- Preserve all lawfully reachable plans — pruning is cognitive limitation (FND-20), not causal suppression
- Make candidate cap a per-agent profile parameter for agent diversity (FND-22)

## Non-Goals

- Hierarchical task decomposition (future architectural change, not needed here)
- Raising `max_node_expansions` as the primary fix (treats the symptom, not the branching factor)
- Modifying candidate generation itself — `search_candidates()` remains unchanged
- Changing the post-expansion beam (`beam_width`) or terminal successor handling

## FOUNDATIONS Alignment

| Principle | How This Spec Satisfies It |
|-----------|----------------------------|
| FND-1 (Maximal Emergence) | Restores behavioral diversity by enabling multi-location plans that were previously unreachable due to budget exhaustion |
| FND-8 (Preconditions, Duration, Cost) | Successor construction still applies full precondition checks, duration estimation, and cost. Pre-expansion scoring only filters *which* candidates enter that pipeline |
| FND-12 (Performance Compresses Computation, Not Causality) | Pre-expansion cap limits what the agent *considers*, not what exists in the world. The highest-scoring candidates (most goal-relevant) are always retained |
| FND-20 (Resource-Bounded Practical Reasoning) | The pre-expansion cap IS the agent's cognitive limit on how many options they evaluate per search step. A shrewd agent considers more options; a simple agent considers fewer |
| FND-22 (Agent Diversity) | `max_candidates_per_expansion` is a per-agent `ExecutionBudget` field with a sensible default |
| FND-28 (No Backward Compatibility) | No shims. New field has a default. Existing scenarios work unchanged |

## Section H — Causal Hooks Declaration

### H.1 — Motivating consequence gap

The planner budget-exhausts on every `AcquireCommodity` goal that requires travel, producing behavioral collapse where all agents cycle sleep+relieve. This blocks: survival (agents die), economic activity (zero trade/crafting), social interaction (no planning headroom), and exploration (travel plans unreachable). The root cause is that 1400–2600 candidates at each expansion consume the entire budget at depth 0.

### H.2 — Entities, relations, records introduced

No new entities or world-state records. The only new stored state is a field on `ExecutionBudget` (per-agent component):
- `max_candidates_per_expansion: u16` — maximum candidates to evaluate per search step

### H.3 — Actions or world processes that mutate them

None. This is a planner-internal optimization. No new actions or world processes.

### H.4 — Information produced, travel, observability

Diagnostic only: `SearchExpansionSummary` gains `candidates_before_scoring: u16` to track how many candidates existed before pre-expansion truncation. This appears in decision traces (debug tooling), not in world state.

### H.5 — Conserved quantities

None affected.

### H.6 — Scarce capacities, contention

None introduced.

### H.7 — Partial failures, aftermath

If the scoring function ranks poorly (unlikely given the tiered approach), an agent may miss a viable plan that would have been found with exhaustive evaluation. This manifests as `BudgetExhausted` — the same failure mode that exists today. The risk is mitigated by the tiered scoring which always preserves goal-terminal and travel-toward-goal candidates.

### H.8 — Positive feedback loops amplified

None.

### H.9 — Physical dampeners

N/A.

### H.10 — Cross-system interaction

None. The change is internal to the planner search loop in `worldwake-ai`.

## Deliverables

### D1: `max_candidates_per_expansion` on `ExecutionBudget`

**File**: `crates/worldwake-core/src/execution_budget.rs`

```rust
pub struct ExecutionBudget {
    pub beam_width: u8,
    pub max_prerequisite_locations: u8,
    pub max_candidates_per_expansion: u16,  // NEW
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self {
            beam_width: 8,
            max_prerequisite_locations: 3,
            max_candidates_per_expansion: 32,  // NEW
        }
    }
}
```

The default of 32 is 4x the `beam_width` (8), providing a generous buffer while cutting 1400+ candidates down to a tractable set.

### D2: Candidate relevance scoring function

**File**: `crates/worldwake-ai/src/search/candidate_scoring.rs` (new module)

A lightweight scoring function that operates on `SearchCandidate` metadata without building successors:

```rust
pub fn candidate_relevance_score(
    candidate: &SearchCandidate,
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    actor_place: Option<EntityId>,
    goal_places: &[EntityId],
    snapshot: &PlanningSnapshot,
) -> u32
```

**Scoring tiers** (additive, lower = higher priority):

| Tier | Score | Condition |
|------|-------|-----------|
| Goal-terminal op | 0 | `op_kind` matches a goal-satisfying operation (Consume for eat/drink goals, Harvest for production goals, Trade for acquisition goals, etc.) |
| Travel toward goal | 100 | `op_kind == Travel` AND target place is in `goal_places` or moves closer to a goal place per `snapshot.min_perceived_travel_cost()` |
| Prerequisite op | 200 | `op_kind` matches a known prerequisite for the goal (QueueForFacilityUse, MoveCargo, DropItem) |
| Travel elsewhere | 300 | `op_kind == Travel` AND target is NOT in goal_places and not closer |
| Unrelated op | 400 | Everything else (Sleep, Relieve, Wash, Patrol, social actions, etc.) |

Goal-terminal op identification uses `PlannerOpSemantics.op_kind` matched against the goal's `GoalKindTag`. The existing `prune_travel_away_from_goal()` already computes spatial alignment for travel candidates — the scoring function reuses that logic without removing candidates (scoring instead of pruning).

### D3: Pre-expansion scoring and truncation in search loop

**File**: `crates/worldwake-ai/src/search/mod.rs`

Insert between candidate generation (line ~155) and the successor-construction loop (line ~182):

```rust
// Pre-expansion candidate scoring and truncation
let pre_expansion_cap = depth_scaled_cap(
    execution_budget.max_candidates_per_expansion,
    depth,
);
if candidates.len() > usize::from(pre_expansion_cap) {
    let candidates_before = candidates.len() as u16;
    score_and_truncate_candidates(
        &mut candidates,
        goal,
        semantics_table,
        actor_place,
        &combined_places.places,
        snapshot,
        pre_expansion_cap,
    );
    // Record for diagnostics
    candidates_before_scoring = candidates_before;
}
```

**Depth-sensitive scaling** (inline formula, no new types):

```rust
fn depth_scaled_cap(base: u16, depth: u8) -> u16 {
    match depth {
        0 => base,                    // full cap at root
        1 => base * 3 / 4,           // 75% at depth 1
        _ => base / 2,               // 50% at depth 2+
    }
}
```

At default cap=32: depth 0 gets 32, depth 1 gets 24, depth 2+ gets 16. These are still 2–4x the beam_width, ensuring ample candidates reach successor construction.

### D4: Diagnostic enrichment

**File**: `crates/worldwake-ai/src/decision_trace.rs`

Add to `SearchExpansionSummary`:

```rust
pub candidates_before_scoring: u16,  // candidates before pre-expansion truncation (0 = no truncation)
```

This field appears in the planner's decision trace output, allowing the observer to report how aggressively the pre-expansion cap is filtering.

### D5: Scenario and AgentDef integration

**File**: `crates/worldwake-cli/src/scenario/types.rs`

`ExecutionBudgetDef` (if it exists) or `AgentDef` gains the `max_candidates_per_expansion` field. Scenarios can override the default per agent.

### D6: Golden test — multi-location plan found within budget

**File**: `crates/worldwake-ai/tests/golden_planner_heuristics.rs` (new)

**Scenario**: 2 agents at a barren location (no food/water). A resource-rich location exists 1–2 hops away with a Well and OrchardRow. Multiple other locations with various affordances (workstations, NPCs, facilities) create a branchy search space. Agents have standard cognitive profiles with default `max_candidates_per_expansion` (32). Run for 300 ticks.

**Assertions**:
1. At least one agent's planner returns `PlanSearchResult::Found` for an `AcquireCommodity` goal containing a Travel step followed by a Harvest or Consume step
2. The plan is found with `expansions_used < max_node_expansions` (not budget-exhausted)
3. `SearchExpansionSummary.candidates_before_scoring > 0` for at least one expansion (truncation was active)

**Regression guard**: A second test with `max_candidates_per_expansion = u16::MAX` on the same scenario demonstrates budget exhaustion, proving the heuristic is necessary.

**Agent diversity test**: Two agents with different `max_candidates_per_expansion` (16 vs 48) both find plans but with different expansion profiles.

## SystemFn Integration

No new SystemFn. The change is internal to `search_plan()` in the planner, which is called from the existing agent decision tick.

## Component Registration

`ExecutionBudget` is already registered on `EntityKind::Agent`. The new field is added to the existing struct with a default value.

## Cross-System Interactions

None. The change is entirely within the `worldwake-ai` planner search loop. It reads from `ExecutionBudget` (core component) and `PlannerOpSemantics` (AI-internal). No cross-system calls.

## Profile-Driven Parameters

| Parameter | Component | Type | Default | Effect |
|-----------|-----------|------|---------|--------|
| `max_candidates_per_expansion` | `ExecutionBudget` | `u16` | 32 | Maximum candidates evaluated per search expansion before depth scaling. Higher = broader search at cost of budget. Lower = more focused search reaching deeper plans. |
