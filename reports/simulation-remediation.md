# Simulation Remediation Proposals

Source report: `reports/simulation-observer-report.md`
Generated: 2026-04-09

---

## Proposed Golden Tests

### GT-1: Agent Travels To Resource When Local Supply Exhausted

**Source finding**: Finding 2 (Action Loops), Finding 5 (Sustained Critical Needs), Finding 10 (Economic Stagnation)
**Severity**: CRITICAL
**File**: `crates/worldwake-ai/tests/golden_ai_decisions.rs` (or new `golden_resource_seeking.rs`)
**Setup**:
- 2 places: ResourcelessCamp (no food/water sources), ResourceVillage (apple source + water source), connected by a road with short travel time
- 1 AI agent at ResourcelessCamp with moderate hunger and thirst (e.g., 500 each)
- Agent has beliefs about ResourceVillage and its resources (seeded via `seed_actor_local_beliefs` or manual belief injection)
- Agent has PerceptionProfile configured
**Assertion**: Within 300 ticks, the agent travels to ResourceVillage and performs eat or drink. If the agent remains at ResourcelessCamp for 200+ ticks doing only sleep/relieve, the test fails.
**Rationale**: The existing `build_commitment_preservation_scenario` in `golden_determinism.rs:578` already tests travel-to-orchard-for-food, but that test focuses on commitment persistence across save/load, not on the planner's ability to generate cross-location resource-seeking plans from scratch. This test directly targets the pathological loop observed in the simulation.

**Note**: This test requires agents to *have beliefs* about the remote resource location. If the test passes with seeded beliefs but the simulation fails without them, the root cause is belief formation (see GT-3), not planning.

### GT-2: Max Consecutive Idle Ticks With Multiple Needs

**Source finding**: Finding 3 (Stuck Agents -- Guard Theron 1019 idle ticks)
**Severity**: CRITICAL
**File**: `crates/worldwake-ai/tests/golden_ai_decisions.rs`
**Setup**:
- Existing test at line 2280 already checks `max_idle < 100` for an agent with unsatisfiable thirst but available food/sleep. Extend or create a sibling test:
- 1 AI agent with ALL needs unsatisfiable at current location (no food, no water, no bed -- only relieve_wilderness available)
- Agent has beliefs about a remote location with resources
**Assertion**: `max_idle < 100` -- even when multiple needs are unsatisfiable, the agent should still attempt *something* (travel, sleep, relieve, or any fallback). A 1019-tick idle streak is never acceptable.
**Rationale**: The existing idle-streak test (line 2378) covers the case where thirst is unsatisfiable but food exists locally. It does NOT cover the case where *nothing* is locally satisfiable except relieve. Theron's 1019-tick shutdown suggests the planner enters a deadlock state where it cannot produce any plan at all.

### GT-3: Perception Forms Beliefs About Place-Graph Neighbors

**Source finding**: Finding 8 (Belief Staleness)
**Severity**: HIGH
**File**: New file `crates/worldwake-ai/tests/golden_perception_beliefs.rs` or add to `golden_perception_exposure.rs`
**Setup**:
- 3 places: StartVillage (water source), Forest (apple source), Camp (nothing), all connected
- 1 AI agent starting at StartVillage with PerceptionProfile
- Run perception system for enough ticks that the agent observes the local water source
**Assertion**: After 50 ticks at StartVillage, the agent's belief store contains beliefs about: (a) the water source entity, (b) the place entity StartVillage itself, and (c) ideally neighboring places (Forest, Camp) if the agent has observed roads/connections.
**Rationale**: The observer report shows agents perceive 91-204 events but their belief summaries contain only Waste items -- no places, no resource sources, no agents. This suggests the perception-to-belief pipeline filters out infrastructure entities. This test protects the invariant that perception *must* form beliefs about resource sources and places, not just portable items.

### GT-4: Agent Diversity -- Different Profiles Produce Different Behavior

**Source finding**: Finding 2 (Action Loops -- all 3 AI agents collapse into identical sleep+relieve pattern)
**Severity**: HIGH
**File**: `crates/worldwake-ai/tests/golden_reasoning_diversity.rs`
**Setup**:
- 1 place with limited resources (e.g., 3 apples, 2 water)
- 3 AI agents with *different* UtilityProfiles (one hunger-prioritizing, one thirst-prioritizing, one fatigue-prioritizing)
- All agents have PerceptionProfile
**Assertion**: Over 200 ticks, the 3 agents do NOT produce identical action sequences. At minimum, their first non-sleep action should differ, or their action distribution should show measurable variance. (Principle 22: Agent Diversity)
**Rationale**: In the observer report, Kael, Merchant Vara, and Forager Lina -- who should have different roles and priorities -- converge on the exact same `sleep*10 + relieve*1` per 100-tick pattern. This violates Principle 22 (Agent Diversity Through Concrete Variation).

**Note**: `golden_reasoning_diversity.rs` already exists -- check whether existing tests cover this specific collapse scenario before adding.

---

## Proposed Tickets

### TK-1: Investigate Planner Deadlock Causing 1019-Tick Idle Streak

**Source finding**: Finding 3 (Stuck Agents -- Guard Theron)
**Priority**: P0
**Crate(s)**: `worldwake-ai` (planner: `search.rs`, `agent_tick.rs`, `tick_step.rs`)
**Description**: Guard Theron goes completely inert for 1019 consecutive ticks (71% of simulation). He stops performing ANY action -- not even sleep or relieve. This suggests the planner enters a state where it cannot produce any valid plan at all, causing the agent to idle indefinitely. The existing `max_idle < 100` golden test (golden_ai_decisions.rs:2378) should catch this but apparently doesn't fire because the test scenario differs from the simulation scenario.

Investigate:
1. Add affordance trace logging to the observer binary to see what `get_affordances` returns for Theron at tick 420+
2. Add failed-plan trace logging to see whether the planner generates goals but fails search, or never generates goals
3. Determine why sleep/relieve (which should always be plannable) are not being selected
4. Fix the root cause -- likely a planner edge case where multiple unsatisfiable high-priority needs block lower-priority but satisfiable actions

**Acceptance criteria**:
- No agent in the cli-evaluation scenario has >100 consecutive idle ticks
- The fix is validated by GT-2 (proposed above)

### TK-2: Perception System Does Not Form Beliefs About Resource Sources or Places

**Source finding**: Finding 8 (Belief Staleness), Finding 10 (Economic Stagnation)
**Priority**: P0
**Crate(s)**: `worldwake-systems` (perception), `worldwake-core` (belief store)
**Description**: The observer report shows agents with 91-204 perception events but belief summaries containing only Waste items. No beliefs about: places, resource sources (apple trees, water wells), workstations, or other agents. This means the perception-to-belief pipeline is either (a) not observing infrastructure entities, or (b) observing them but not converting observations into retrievable beliefs.

Without beliefs about resource locations and place-graph topology, the planner CANNOT generate multi-location plans (travel to X, pick up food, eat). This is the root cause of findings 2, 5, 6, and 10.

Investigate:
1. What entity types does the perception system observe? Does it filter by EntityKind?
2. Are resource sources (workstations with ResourceSource) being perceived but not stored as beliefs?
3. Are place entities perceived? Do agents form beliefs about the place graph?
4. Is the belief store capacity too small, causing infrastructure beliefs to be evicted in favor of Waste items?

**Acceptance criteria**:
- After spending 50+ ticks at a location with a resource source, the agent's belief store contains a belief about that resource source
- After observing a road/connection to another place, the agent has a belief about that place's existence
- Validated by GT-3

### TK-3: Planner Cannot Generate Travel-to-Resource Plans

**Source finding**: Finding 2 (Action Loops), Finding 6 (Unaddressed Needs), Finding 10 (Economic Stagnation)
**Priority**: P1 (blocked by TK-2 -- beliefs must exist first)
**Crate(s)**: `worldwake-ai` (planner: `candidate_generation.rs`, `search.rs`, `affordance_query.rs`)
**Description**: Even if TK-2 is fixed and agents form beliefs about remote resources, the planner may still lack the ability to chain `travel -> pick_up -> eat/drink` into a multi-step plan. The observer report shows agents never attempt travel after the initial early-phase movement, even though resources exist 1 hop away.

Investigate:
1. Does `generate_candidates` emit Eat/Drink goals when no food/water is at the current location but the agent believes food/water exists elsewhere?
2. Does `search_plan` find plans that include travel as an enabling step? Check whether travel appears as a valid operator in the GOAP search space.
3. Does the `get_affordances` output include "travel to Place X" when the agent has beliefs about Place X?
4. Is there a planner depth limit that prevents 3+ step plans (travel -> harvest/pick_up -> consume)?

**Acceptance criteria**:
- An agent with beliefs about food at a remote location and critical hunger generates a plan including travel + eat
- Validated by GT-1

### TK-4: Add Affordance and Failed-Plan Tracing to Observer Binary

**Source finding**: Trace Quality Assessment (observer report limitations)
**Priority**: P1
**Crate(s)**: `worldwake-ai` (decision tracing), `worldwake-cli` (observer binary)
**Description**: The observer report identified a critical blind spot: no affordance trace (what the planner considered and rejected) and no failed-plan trace (what plans were attempted but failed search). These traces would directly explain findings 2, 3, 5, and 6.

Add to the observer dump:
1. Per-agent affordance summary: which affordances were available at each decision point
2. Per-agent failed-goal summary: goals that were generated but had no valid plan
3. Per-agent plan-rejection summary: plans that failed search and why (no operators, precondition failure, depth limit)

**Acceptance criteria**:
- Observer dump includes a "Planning Trace" section per agent showing generated goals, attempted plans, and failure reasons
- Re-running the cli-evaluation scenario produces enough trace data to diagnose the root cause of findings 2, 3, and 6

---

## Findings Deferred or Not Requiring Independent Remediation

| Finding | Severity | Reason for Deferral |
|---------|----------|---------------------|
| 1. Redundant Perception | MEDIUM | Architecturally expected (perception fires on events, not state changes). Optimization candidate but not a correctness issue. Revisit after core belief/planning fixes land. |
| 4. Failed Action Spirals | LOW | Theron's 21->4 investigate ratio is concentrated in ticks 0-99 during early patrol phase. Likely target-availability issues during rapid-fire attempts. Not pathological -- revisit if pattern persists after TK-1 fix. |
| 5. Sustained Critical Needs | CRITICAL | Downstream symptom of Finding 8 (no resource beliefs) and Finding 2 (no travel-to-resource plans). Deferred to TK-2 + TK-3. |
| 6. Unaddressed Needs | CRITICAL | Downstream symptom of Finding 8. Vara never eats/drinks because the planner has no beliefs about food/water locations. Deferred to TK-2 + TK-3. |
| 7. Impossible Knowledge | NONE | No issues found. |
| 9. Social Isolation | HIGH | Lina's complete lack of social actions may indicate a profile configuration issue (missing tell affordance). However, the broader social absence is likely secondary to the resource crisis -- agents stuck in survival loops don't socialize. Revisit after TK-1/TK-2/TK-3. |
| 10. Economic Stagnation | CRITICAL | Downstream of Finding 8 (no resource beliefs) and Finding 2 (no travel). The economy cannot function if agents never travel to production sites. Deferred to TK-2 + TK-3. |

---

## Summary

| Type | Count | Severity Breakdown |
|------|-------|--------------------|
| Golden Tests | 4 | 2 CRITICAL, 2 HIGH |
| Spec Changes | 0 | -- |
| Tickets | 4 | 2 P0, 2 P1 |
| Deferred | 6 | 3 CRITICAL (downstream), 1 HIGH, 1 MEDIUM, 1 NONE |

### Root Cause Chain

The simulation's failures trace to a single causal chain:

1. **Perception does not form beliefs about resource sources or places** (TK-2) -- this is the root cause
2. **Without resource/place beliefs, the planner cannot generate travel-to-resource plans** (TK-3)
3. **Without travel-to-resource plans, agents collapse into sleep+relieve loops** (GT-1, GT-4)
4. **A separate planner deadlock causes complete agent shutdown** (TK-1, GT-2)

Fix TK-2 first, then TK-3, then validate with GT-1 through GT-4. TK-4 (tracing) can proceed in parallel to aid debugging.

### FOUNDATIONS Alignment

- **Principle 7 (Locality)**: The simulation correctly enforces information locality -- agents don't cheat. But the perception system isn't providing enough *local* information for agents to reason about the world. Locality without perception is blindness.
- **Principle 14 (World State Is Not Belief State)**: Correctly enforced. The problem is that belief state is too impoverished, not that agents access world state.
- **Principle 20 (Resource-Bounded Practical Reasoning)**: The planner should generate multi-step plans (travel -> acquire -> consume) when local solutions don't exist. Currently it appears limited to single-location plans.
- **Principle 22 (Agent Diversity)**: Violated -- all agents collapse into identical behavior patterns despite different profiles.
