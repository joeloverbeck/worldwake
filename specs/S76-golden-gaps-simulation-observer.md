# S76: Golden E2E Gaps -- Simulation Observer Report

**Status**: Draft

## Summary

The simulation observer report (2026-04-09) revealed four behavioral patterns with no golden test coverage: (1) agents failing to travel to remote resources when local supply is exhausted, (2) agents idling for 1000+ ticks when multiple needs are locally unsatisfiable, (3) agents not demonstrating diversity despite different utility profiles, and (4) no test verifying perception forms beliefs about resource sources and places. This spec adds golden scenarios that protect against regression of these emergent behaviors.

## Phase

Phase 7: Consequence Carriers (adjunct)

## Crates

- `worldwake-ai` (golden tests)

## Dependencies

- No spec dependencies. All required infrastructure (travel, perception, belief store, utility profiles) is already implemented.

## Design Goals

- Cover the four behavioral gaps identified by the simulation observer.
- Each scenario exercises a multi-system chain, not a single unit.
- Tests should fail for the specific observed pathology (not just "agent does something").

## Non-Goals

- Fixing the root cause of impoverished beliefs (that is S77).
- Fixing the planner (the planner is architecturally sound; the sim failures are belief-driven).
- Observer tooling enhancements (that is S78).

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P1 (Maximal Emergence) | All scenarios verify emergent behavior chains, not scripted outcomes |
| P7 (Locality) | GT-A and GT-C verify information flows through perception and belief, not global state |
| P14 (World State Is Not Belief State) | GT-C specifically verifies agents form beliefs from perception, not world state access |
| P20 (Resource-Bounded Practical Reasoning) | GT-A and GT-B verify the planner generates multi-step plans under belief constraints |
| P22 (Agent Diversity Through Concrete Variation) | GT-D directly exercises this principle |
| P26 (Systems Interact Through State) | All scenarios chain 3+ systems through state, not direct calls |
| P29 (Debuggability) | Each scenario's assertion targets a specific emergent chain link |

## Section H: Causal Hooks

No new causal hooks. All tested behaviors use existing systems.

### Information-Path Analysis

- GT-A: Hunger/thirst need state -> candidate generation -> planner search -> travel + consume action chain. Beliefs about remote resources must exist (seeded or perceived).
- GT-B: Multiple unsatisfiable needs -> candidate generation -> planner must still find SOME valid plan (sleep, relieve, travel). Fallback from top-priority unsatisfiable to lower-priority satisfiable.
- GT-C: Colocated entity observation -> `build_observed_entity_snapshot()` -> `record_observed_snapshot()` -> `entity_claims_for_snapshot()` -> belief store. Resource source beliefs survive `enforce_capacity()`.
- GT-D: Different `UtilityProfile` weights -> different goal rankings in `rank_goals()` -> different plan selections -> different action sequences.

### Positive-Feedback Analysis

No positive-feedback loops introduced. These are test scenarios only.

### Concrete Dampeners

N/A (no feedback loops).

### Stored State vs. Derived

No new stored state or derived views. Tests only.

---

## Proposed Scenarios

### Scenario S76-A: Agent Travels To Remote Resource When Local Supply Exhausted

**Source finding**: Simulation observer Finding 2 (Action Loops), Finding 5 (Sustained Critical Needs)

**Description**: An agent at a resource-poor location with seeded beliefs about a remote resource location plans and executes a travel + consume sequence when hunger is critical and no food exists locally.

**Setup**:
- 2 places: BarrenCamp (no food/water sources), ResourceVillage (apple source + water source), connected by road with short travel time
- 1 AI agent at BarrenCamp with moderate hunger (pm(500)) and thirst (pm(500))
- Agent has PerceptionProfile
- Agent has seeded beliefs about ResourceVillage and its apple source (via `seed_actor_local_beliefs` or manual belief injection)

**Assertion**: Within 300 ticks, the agent travels to ResourceVillage and performs eat or drink. If the agent remains at BarrenCamp for 200+ ticks doing only sleep/relieve, the test fails.

**GoalKinds exercised**: `SatisfyNeed` (Hunger or Thirst), Travel (as enabling step)

**ActionDomains exercised**: Travel, Metabolism (eat/drink)

**Systems exercised**: AI (candidate generation with remote acquisition paths), planner search (multi-step travel + consume), Travel action, Eat/Drink action

**What emergence it demonstrates**: The agent's decision to travel is not scripted -- it emerges from: critical need state -> candidate generation scanning remote beliefs -> planner chaining travel + acquire + consume. Three systems (needs, AI, travel) chain through state.

**Foundation principle alignment**:
- P1 (Maximal Emergence): Travel decision emerges from need + belief + planning
- P7 (Information Locality): Agent travels based on beliefs, not global knowledge
- P14 (World State Is Not Belief State): Test seeds beliefs, not world-state access
- P20 (Resource-Bounded Practical Reasoning): Multi-step plan generation

**Why it is not a duplicate**: `golden_multi_hop_travel_plan()` (golden_ai_decisions.rs:1561) tests travel-to-orchard when the agent starts with hunger, but the agent starts at a location with road access to the orchard and the test focuses on multi-hop path selection. This scenario tests the specific pathology observed in simulation: agent at a barren location with remote resource beliefs failing to leave.

**Note**: This test requires agents to have beliefs about the remote resource location. If the test passes with seeded beliefs but the simulation fails without them, the root cause is belief formation (S77), not planning.

---

### Scenario S76-B: Max Consecutive Idle With All Local Needs Unsatisfiable

**Source finding**: Simulation observer Finding 3 (Guard Theron 1019 idle ticks)

**Description**: An agent at a location where no needs except relieve_wilderness are locally satisfiable must still attempt actions (travel, sleep, relieve) rather than entering a prolonged idle state.

**Setup**:
- 2 places: BarrenOutpost (no food, no water, no bed -- only relieve_wilderness available), ResourceTown (all resources), connected by road
- 1 AI agent at BarrenOutpost with moderate hunger (pm(500)), thirst (pm(500)), fatigue (pm(500)), bladder (pm(500))
- Agent has PerceptionProfile
- Agent has seeded beliefs about ResourceTown and its resources

**Assertion**: Over 300 ticks, `max_consecutive_idle < 100`. Even when multiple needs are locally unsatisfiable, the agent should attempt relieve, sleep, or travel to ResourceTown. A 1019-tick idle streak is never acceptable.

**GoalKinds exercised**: `SatisfyNeed` (multiple), Travel

**ActionDomains exercised**: Travel, Metabolism, Physiology (relieve)

**Systems exercised**: AI (candidate generation fallback from unsatisfiable top-priority to satisfiable lower-priority), planner search, exhaustion cache budget backoff

**What emergence it demonstrates**: When the top-priority need is unsatisfiable locally, the planner falls back to lower-priority satisfiable actions or generates travel plans. The exhaustion cache prevents infinite thrashing. The agent remains active despite adversity.

**Foundation principle alignment**:
- P20 (Resource-Bounded Practical Reasoning): Planner generates fallback plans under constraint
- P22 (Agent Diversity): Agent responds to adversity through concrete parameters, not shutdown

**Why it is not a duplicate**: The existing `golden_fallback_to_addressable_need_when_top_need_unsatisfiable()` (golden_ai_decisions.rs:2263) tests partial unsatisfiability: thirst is unsatisfiable but food is available locally. This scenario tests total local unsatisfiability where the only local option is relieve_wilderness and all other needs require travel.

---

### Scenario S76-C: Perception Forms Beliefs About Resource Sources

**Source finding**: Simulation observer Finding 8 (Belief Staleness)

**Description**: An agent spending time at a location with resource sources forms beliefs about those resource sources, not just about portable items.

**Setup**:
- 1 place: FarmVillage with an apple source entity (EntityKind with ResourceSource component) and a water source entity
- 1 AI agent at FarmVillage with PerceptionProfile
- Several ground items (e.g., waste) also present to test belief capacity competition

**Assertion**: After 50 ticks at FarmVillage, the agent's `AgentBeliefStore.known_entities` contains beliefs about: (a) the apple source entity, and (b) the water source entity. If the belief store contains only Waste/ground-item beliefs and no resource source beliefs, the test fails.

**GoalKinds exercised**: N/A (perception-focused, not planning-focused)

**ActionDomains exercised**: N/A

**Systems exercised**: Perception system, belief store recording, `enforce_capacity()` eviction

**What emergence it demonstrates**: The perception-to-belief pipeline must preserve beliefs about infrastructure entities (resource sources) even in the presence of many ground items competing for belief capacity. This is the foundation for all resource-seeking behavior.

**Foundation principle alignment**:
- P7 (Information Locality): Agent learns about resources through local perception
- P14 (World State Is Not Belief State): Beliefs formed through observation, not world-state access
- P15 (Knowledge Acquired Locally): Resource knowledge enters through perception
- P16 (Ignorance and Contradiction): Belief capacity is finite; the test verifies that capacity allocation prioritizes actionable beliefs

**Why it is not a duplicate**: `golden_perception_exposure.rs` scenarios 116-119 test perception modulation (fidelity, concealment, fatigue, attention cost). None verify that resource source entities survive the belief pipeline and appear in `known_entities`.

---

### Scenario S76-D: Different Utility Profiles Produce Different Goal Orderings

**Source finding**: Simulation observer Finding 2 (all 3 AI agents collapse into identical sleep+relieve pattern)

**Description**: Agents with different UtilityProfile weights produce different action sequences in the same environment, demonstrating that profile diversity drives behavioral diversity.

**Setup**:
- 1 place with limited resources (3 apple lots, 2 water lots, 1 bed)
- 3 AI agents with different UtilityProfiles:
  - Agent A: hunger-prioritizing (high hunger utility weight)
  - Agent B: thirst-prioritizing (high thirst utility weight)
  - Agent C: fatigue-prioritizing (high fatigue utility weight)
- All agents have PerceptionProfile
- All agents start with moderate levels of all needs (pm(500) each)

**Assertion**: Over 200 ticks, the 3 agents do NOT produce identical action sequences. At minimum, their first non-relieve action should differ, or their action distribution (eat vs. drink vs. sleep counts) should show measurable variance. Identical `sleep*10 + relieve*1` patterns across all 3 agents fail the test.

**GoalKinds exercised**: `SatisfyNeed` (Hunger, Thirst, Fatigue)

**ActionDomains exercised**: Metabolism (eat, drink), Rest (sleep), Physiology (relieve)

**Systems exercised**: AI (goal ranking via `rank_goals()` using UtilityProfile weights), candidate generation, planner search

**What emergence it demonstrates**: Different utility weights cause different goal rankings, leading to different plan selections and action sequences. This is Principle 22 in action: diversity from concrete parameter variation, not from scripted role differences.

**Foundation principle alignment**:
- P22 (Agent Diversity Through Concrete Variation): Directly exercises this principle
- P20 (Resource-Bounded Practical Reasoning): Each agent reasons from their own priorities
- P1 (Maximal Emergence): Behavioral diversity emerges from parameter variation

**Why it is not a duplicate**: `golden_reasoning_diversity.rs` tests search-depth divergence (different `CognitiveProfile.max_node_expansions`). It does NOT test utility-profile divergence (different `UtilityProfile` weights producing different goal orderings).

---

## Ticket Breakdown

### S76-001: Implement Scenarios S76-A and S76-B (planner fallback coverage)

**File**: `crates/worldwake-ai/tests/golden_ai_decisions.rs` (or new `golden_simulation_gaps.rs` if file is too large)

**Tasks**:
1. Build S76-A scenario: 2 places, 1 agent with seeded remote beliefs, moderate hunger/thirst
2. Assert agent travels to ResourceVillage and eats/drinks within 300 ticks
3. Build S76-B scenario: 2 places, 1 agent with all local needs unsatisfiable except relieve
4. Assert `max_consecutive_idle < 100` over 300 ticks
5. Add `// Scenario S76-A` and `// Scenario S76-B` metadata headers
6. Add deterministic replay companions (`*_replays_deterministically`)

### S76-002: Implement Scenario S76-C (perception belief coverage)

**File**: `crates/worldwake-ai/tests/golden_perception_exposure.rs` (or new file if too large)

**Tasks**:
1. Build scenario: 1 place with resource sources + ground waste items
2. Run perception for 50 ticks
3. Assert `AgentBeliefStore.known_entities` contains resource source entities
4. Assert resource source beliefs survive `enforce_capacity()` in the presence of ground items
5. Add `// Scenario S76-C` metadata header
6. Add deterministic replay companion

### S76-003: Implement Scenario S76-D (utility profile diversity)

**File**: `crates/worldwake-ai/tests/golden_reasoning_diversity.rs`

**Tasks**:
1. Build scenario: 1 place with limited resources, 3 agents with different UtilityProfiles
2. Run for 200 ticks, collect per-agent action distributions
3. Assert action distributions are not identical across all 3 agents
4. Add `// Scenario S76-D` metadata header
5. Add deterministic replay companion

## Replay and Conservation Requirements

- Each primary golden scenario MUST have a `*_replays_deterministically` companion
- Conservation verification: No new physical goods introduced. `verify_conservation` should pass unchanged.
- All scenarios must use `ChaCha8Rng` seeded determinism
