# Simulation Remediation Proposals

Source report: `reports/simulation-observer-report.md`
Generated: 2026-04-10

## Context

The simulation ran 1440 ticks (1 simulated day) with 4 agents across 5 places. The dominant failure mode is geographic: 3 of 4 agents migrated to Dusty Trail (resource-poor) early in the simulation and became trapped. The GOAP planner's `AcquireCommodity` goal generates 1000-7000+ candidates for multi-location resource acquisition but the expansion budget caps at 224-300, causing systematic budget exhaustion. This single root cause drives 5 of 8 findings (Action Loops, Stuck Agents, Sustained Critical Needs, Unaddressed Needs, Economic Stagnation). The remaining findings are either independent (waste/carry capacity, belief staleness) or partially downstream (social isolation).

No prior `reports/simulation-remediation.md` existed -- this is the first run.

---

## Proposed Golden Tests

### GT-1: Resource Affordance At Source Location
**Source finding**: Finding 6 (Unaddressed Needs)
**Severity**: CRITICAL
**File**: `crates/worldwake-ai/tests/golden_simulation_gaps.rs`
**Setup**: Single agent at a location with a water resource source (e.g., Well facility). Agent has high thirst, no water in inventory. Location has harvestable water.
**Assertion**: Within 100 ticks, agent generates a `drink` affordance (or `pick_up` + `drink` chain) and successfully drinks. Verify the affordance query includes drink/pick_up when consumable resources exist at the agent's location.
**Rationale**: The observer report shows Merchant Vara at Thornwall Village (tick 0, which has a Well) had no `drink` affordance. This test validates that resource sources at the agent's location produce the expected consumption affordances.
**Existing coverage**: `golden_thirst_driven_acquisition` tests thirst-driven water acquisition but may assume water is already in inventory. `golden_wash_action` gives water directly to the agent. Neither tests the "resource source at location -> affordance" path specifically.

---

## Proposed Spec Changes

### SC-1: Waste Disposal Action
**Source finding**: Finding 4 (Failed Action Spirals), Cross-Cutting Pattern 5 (Waste Economy)
**Spec**: New spec needed -- no existing spec covers item disposal
**Section**: New spec: "Waste Disposal and Inventory Management"
**Change**: Define a `drop_item` or `discard` action that allows agents to remove unwanted items (specifically Waste) from inventory, placing them on the ground at the agent's current location. The action must have preconditions (agent holds item), duration (non-instant per FND-08), cost (occupies attention), and aftermath (item exists on ground, not destroyed -- per FND-04 and FND-10). Additionally, the planner needs a goal type (e.g., `FreeCarryCapacity` or `DisposeWaste`) that triggers when carry capacity is near-full with low-value items.
**FOUNDATIONS alignment**:
- FND-08: Every Action Has Preconditions, Duration, Cost, and Occupancy -- the drop action must not be free/instant
- FND-10: Outcomes Leave Aftermath -- dropped waste exists on the ground, doesn't vanish
- FND-11: Every Positive Feedback Loop Needs Physical Dampener -- waste accumulation currently has no dampener; disposal is the physical dampener
- FND-04: Persistent Identity -- waste items persist after being dropped, maintaining object permanence

### SC-2: Belief-Informed Planner Candidate Pruning
**Source finding**: Finding 5 (Sustained Critical Needs) -- ROOT CAUSE for Findings 2, 3, 5, 6, 9
**Spec**: New spec or addendum to planner architecture documentation
**Section**: Search space management for multi-location plans
**Change**: The `AcquireCommodity` goal generates candidates across all known locations, producing 1000-7000+ candidates that exceed the 224-300 expansion budget. The spec should define one or more of:
1. **Belief-informed pruning**: During candidate generation, only consider places the agent has beliefs about (knows exist, knows has resources). Agents with no beliefs about remote resources should not generate remote acquisition candidates.
2. **Hierarchical decomposition**: Decompose `AcquireCommodity` at a remote location into `TravelTo(location)` + `AcquireLocal(commodity)` subgoals, reducing search depth per subgoal.
3. **Profile-driven budget scaling**: Allow `CognitiveProfile` to influence expansion budget (FND-22 agent diversity), so "smarter" agents can search deeper.

The spec must also address the "return journey" gap: once an agent is at a resource-poor location, it should be able to generate a `TravelTo(resource-rich location)` goal based on beliefs about where resources exist.
**FOUNDATIONS alignment**:
- FND-20: Resource-Bounded Practical Reasoning -- planner budget should be tunable and belief-informed, not an arbitrary wall
- FND-01: Maximal Emergence -- multi-location planning should emerge from agent beliefs about the world
- FND-14: World State Is Not Belief State -- candidate pruning uses beliefs (what agent knows), not world state
- FND-22: Agent Diversity -- budget/strategy differences between agents create behavioral diversity

---

## Proposed Tickets

### TK-1: Investigate and Fix AcquireCommodity Budget Exhaustion
**Source finding**: Finding 5 (Sustained Critical Needs) -- ROOT CAUSE
**Priority**: P0
**Crate(s)**: `worldwake-ai` (search/mod.rs, search/transition.rs, candidate_generation.rs)
**Description**: `AcquireCommodity{Water}` at Dusty Trail generates 1000-7000+ candidates across 224-300 expansion budget, consistently returning `budget-exhausted`. Three of four agents starve because the planner cannot find multi-location resource acquisition plans. This is the single root cause for Findings 2, 3, 5, 6, and 9.

Concrete investigation steps:
1. Instrument search to log the plan graph shape for Water acquisition from a 2-place topology (Dusty Trail + Thornwall Village with Well)
2. Identify where branching explodes -- likely `combined_relevant_places` generating candidates across all locations at each depth
3. Implement belief-informed pruning so agents only consider places they have beliefs about
4. Verify `golden_remote_travel_when_local_supply_exhausted` and `golden_max_idle_under_remote_resource_scarcity` still pass
**Dependencies**: None (this is the root cause)
**FOUNDATIONS alignment**: FND-20 (Resource-Bounded Practical Reasoning), FND-29 (Debuggability)
**Acceptance criteria**:
- Agent at resource-poor location with beliefs about a resource-rich location can plan `TravelTo + AcquireLocal` within expansion budget
- `golden_remote_travel_when_local_supply_exhausted` passes
- `golden_max_idle_under_remote_resource_scarcity` passes with idle < 100 ticks
- New golden test: agent at barren location with belief about remote water source commits travel within 200 ticks

### TK-2: Fix "Unknown Location" Belief for Current Place
**Source finding**: Finding 7 (Belief Staleness)
**Priority**: P1
**Crate(s)**: `worldwake-ai` (belief system), `worldwake-systems` (perception)
**Description**: Agents at Dusty Trail believe the location of Dusty Trail is "Unknown location." An agent physically present at a place should know where it is -- presence is the most local form of knowledge (FND-15). Investigate whether:
- The perception system fires a self-location belief update on arrival
- Place entities have a "location" concept that differs from agent location (places don't have locations in the same sense)
- The "Unknown location" string is an artifact of how place-entity locations are represented in beliefs
**Dependencies**: None
**FOUNDATIONS alignment**: FND-14 (World State Is Not Belief State -- but presence implies knowledge), FND-15 (Knowledge Is Acquired Locally)
**Acceptance criteria**:
- Agent at a place has a belief about that place's identity (not "Unknown location")
- If "Unknown location" is expected for place entities (they don't have a location-of-a-location), then the observer should display this more clearly

### TK-3: Investigate ShareBelief Frontier Exhaustion
**Source finding**: Finding 8 (Social Isolation)
**Priority**: P2
**Crate(s)**: `worldwake-ai` (search, goal dispatch for ShareBelief)
**Description**: ShareBelief goal consistently frontier-exhausts at depth 0 with 1 expansion. This means no operators are available for the ShareBelief goal. 14 of 20 failed plans for Kael are frontier-exhausted at depth 0. Investigate whether:
- The `tell` action operator requires a co-located listener and none is available (unlikely -- agents are co-located at Dusty Trail)
- There is a cooldown or "already told" filter that blocks all candidates
- The goal formulation doesn't match any operator's preconditions
**Dependencies**: None
**FOUNDATIONS alignment**: FND-20 (Resource-Bounded Reasoning -- don't waste budget on infeasible goals), FND-29 (Debuggability)
**Acceptance criteria**:
- ShareBelief plans either succeed when a co-located listener exists, or are pruned early with a clear rejection reason (not frontier-exhausted at depth 0)

### TK-4: Add Frontier-Exhaustion Rejection Reasons to Observer Output
**Source finding**: TQ-6 (Trace Quality Assessment)
**Priority**: P1
**Crate(s)**: `worldwake-cli` (observer binary), `worldwake-ai` (decision_trace.rs)
**Description**: When a plan search returns `FrontierExhausted`, the observer output shows only "frontier-exhausted (1 expansion, 0 depth)" with no explanation of why no operators matched. The `SearchExpansionSummary` already tracks `candidates_generated`, `candidates_skipped`, `terminal_successors`, and `non_terminal_after_beam` per depth. Surface a human-readable summary: "no applicable operators: tell requires co-located listener" or "all candidates pruned by beam at depth 3."
**Dependencies**: None (aids debugging of TK-1 and TK-3)
**FOUNDATIONS alignment**: FND-29 (Debuggability Is Product Feature)
**Acceptance criteria**:
- Observer output for frontier-exhausted plans includes the rejection reason
- At minimum: "N operators checked, all failed precondition X" or "0 operators applicable for goal type Y"

### TK-5: Add Per-Agent Need Snapshots at Behavioral Transitions
**Source finding**: TQ-1 (Trace Quality Assessment)
**Priority**: P2
**Crate(s)**: `worldwake-cli` (observer binary)
**Description**: The observer currently captures needs only in end-state summaries (min/max/avg and ticks-above-750). Add per-agent need snapshots (hunger, thirst, fatigue, bladder, dirtiness values) at behavioral transition points -- specifically when the action type count drops by 50%+ between consecutive time bins, or when a new goal is adopted.
**Dependencies**: None
**FOUNDATIONS alignment**: FND-29 (Debuggability)
**Acceptance criteria**:
- Observer output includes need values at detected behavioral transitions
- At minimum: "Tick 500: action repertoire narrowed (5 types -> 2 types). Needs: hunger=750, thirst=800, fatigue=200, bladder=100, dirtiness=500"

### TK-6: Emit Affordance Snapshots After Travel
**Source finding**: TQ-2 (Trace Quality Assessment)
**Priority**: P2
**Crate(s)**: `worldwake-cli` (observer binary)
**Description**: Affordances are currently shown only at tick 0. Agents that travel have different affordances at their new location. Emit affordance snapshots at configurable intervals (e.g., every 200 ticks) and after travel action commits.
**Dependencies**: None
**FOUNDATIONS alignment**: FND-29 (Debuggability)
**Acceptance criteria**:
- Observer output includes affordance snapshots after travel commits
- At minimum: end-of-simulation affordances shown in per-agent summary

### TK-7: Surface Exact Death Tick in Observer Output
**Source finding**: TQ-3 (Trace Quality Assessment), Finding 3 (Stuck Agents)
**Priority**: P2
**Crate(s)**: `worldwake-cli` (observer binary)
**Description**: Guard Theron has 1 dead tick but the exact death tick isn't stated in the observer output. The `DeadAt` component exists in the engine but the observer binary doesn't surface the tick prominently in the per-agent summary. Add "Died at tick N (cause: X)" to the agent summary header.
**Dependencies**: None
**FOUNDATIONS alignment**: FND-29 (Debuggability), FND-10 (Outcomes Are Granular)
**Acceptance criteria**:
- Per-agent summary shows "Died at tick N (cause: starvation/dehydration/combat/etc.)" when applicable

---

## Findings Deferred or Not Requiring Independent Remediation

| Finding | Severity | Reason for Deferral |
|---------|----------|---------------------|
| Finding 1: Redundant Perception | LOW | Expected behavior -- perception fires on events, not selectively on state changes. Agents' state changes continuously, so repeated observations may carry new information. No remediation needed. |
| Finding 2: Action Loops | HIGH | Downstream symptom of Root Cause A (planner budget exhaustion). Agents collapse to sleep+relieve because the planner can't find resource acquisition plans. Deferred to TK-1 (planner fix) + SC-2 (belief-informed pruning). Existing test `golden_max_idle_under_remote_resource_scarcity` covers the invariant. |
| Finding 3a: Guard Theron 1019 idle ticks | CRITICAL | Downstream symptom of Root Cause A. Theron at Dusty Trail with no resources and no beliefs about remote sources. Deferred to TK-1 + SC-2. |
| Finding 4b: Repeated pick_up failures | MEDIUM | Downstream of Finding 4a (waste fills inventory). Once SC-1 (waste disposal action) is implemented, agents can free carry capacity. |
| Finding 6: Unaddressed Needs (no eat/drink) | CRITICAL | Downstream symptom of Root Cause A. At Dusty Trail, no local food/water exists (correct behavior). The issue is the planner can't plan travel to resource-rich locations. Deferred to TK-1 + SC-2. |
| Finding 7b: Stale waste counts in beliefs | MEDIUM | Expected behavior per FND-14 (agents can be wrong). Belief memory capacity (16 entities) means waste counts are partial snapshots. Will improve once SC-1 reduces waste accumulation. |
| Finding 8a: Social activity ceased after tick ~400 | MEDIUM | Downstream symptom of Root Cause A. Survival needs dominate and can't be satisfied, so planner deadlocks. Once agents can address needs, social activity should resume. Existing test `golden_survival_needs_suppress_social_goals` validates that survival needs correctly suppress social goals when critical. |
| Finding 9: Economic Stagnation | CRITICAL | Downstream symptom of Root Cause A. SellCommodity blocked 698 times because buyers and sellers are at different locations and can't plan travel. Resources at Thornwall Village go untouched. Deferred to TK-1 + SC-2. |
| TQ-4: Waste not distinguished in belief counts | N/A | Acceptable trade-off per observer report. Minor clarity issue -- cross-reference with end-state inventory. |
| TQ-5: No planner budget configuration shown | N/A | Acceptable trade-off per observer report. Inferable from failed plan data. |

---

## Summary

| Type | Count | Severity Breakdown |
|------|-------|--------------------|
| Golden Tests | 1 | 1 CRITICAL |
| Spec Changes | 2 | 1 CRITICAL (SC-2), 1 MEDIUM (SC-1) |
| Tickets | 7 | 1 P0 (TK-1), 2 P1 (TK-2, TK-4), 4 P2 (TK-3, TK-5, TK-6, TK-7) |
| Deferred | 10 | 3 CRITICAL (downstream), 1 HIGH (downstream), 4 MEDIUM (downstream/expected), 1 LOW (expected), 1 N/A |

### Root Cause Chain

```
ROOT CAUSE A: Planner budget exhaustion for multi-location resource acquisition
  Remediation: TK-1 (P0 investigation/fix) + SC-2 (spec for belief-informed pruning)
  |
  |-- Finding 2 (Action Loops) ---- DEFERRED
  |-- Finding 3 (Stuck Agents) ---- DEFERRED (death display: TK-7)
  |-- Finding 5 (Sustained Needs) - ROOT (also: GT-1 affordance test)
  |-- Finding 6 (Unaddressed Needs) DEFERRED
  |-- Finding 9 (Economic Stagnation) DEFERRED
  |-- Finding 8a (Social Isolation) - DEFERRED

ROOT CAUSE B: No waste disposal action
  Remediation: SC-1 (spec for drop/discard action)
  |
  |-- Finding 4 (Failed Action Spirals)
  |-- Finding 7b (Stale waste counts) - partially downstream

INDEPENDENT:
  Finding 7a ("Unknown location" bug) --> TK-2 (P1)
  Finding 8b (ShareBelief frontier-exhausted) --> TK-3 (P2)
  Finding 1 (Redundant Perception) --> No remediation (expected)
```

**Implementation order**:
1. **First (parallel)**: TK-1 (planner fix, P0) + TK-4 (frontier-exhaustion reasons, P1 -- aids TK-1 debugging)
2. **Second (parallel)**: SC-2 (planner spec) + TK-2 (belief location fix, P1) + SC-1 (waste disposal spec)
3. **Third (parallel)**: GT-1 (affordance test) + TK-3 (ShareBelief investigation) + TK-5/TK-6/TK-7 (observer improvements)
4. **Re-run observer**: After TK-1 lands, re-run `/simulation-observer` to verify downstream findings (2, 3, 6, 8a, 9) are resolved

### FOUNDATIONS Alignment

| Principle | Status | Notes |
|-----------|--------|-------|
| FND-01 (Maximal Emergence) | VIOLATED | Agents cannot produce emergent multi-location resource chains because the planner budget prevents it. SC-2 addresses this. |
| FND-04 (Persistent Identity) | OK | Waste items persist correctly. SC-1 extends this to dropped items. |
| FND-06 (World Runs Without Observers) | VIOLATED | 3 of 4 agents become non-functional without human intervention. Root cause is planner, not principle violation per se, but the effect is a frozen world. TK-1 addresses this. |
| FND-08 (Preconditions/Duration/Cost) | OK | Actions correctly have preconditions. The issue is planner inability to chain them. |
| FND-10 (Outcomes Leave Aftermath) | OK | Waste as aftermath works correctly. Death aftermath needs better observability (TK-7). |
| FND-11 (Physical Dampener) | VIOLATED | Waste accumulation has no physical dampener -- no disposal mechanism exists. SC-1 addresses this. |
| FND-14 (World State != Belief State) | OK | Belief staleness is expected. "Unknown location" bug (TK-2) is a minor violation. |
| FND-15 (Knowledge Acquired Locally) | PARTIAL | Agents know their own location physically but may not have a belief about it (TK-2). |
| FND-20 (Resource-Bounded Reasoning) | VIOLATED | Planner budget is a hard wall rather than belief-informed bounded search. SC-2 addresses this. |
| FND-22 (Agent Diversity) | OK | Agents have different profiles. Budget scaling (SC-2) would enhance diversity. |
| FND-29 (Debuggability) | PARTIAL | Decision traces exist but lack frontier-exhaustion reasons (TK-4), need snapshots (TK-5), affordance snapshots (TK-6), and death tick display (TK-7). |
