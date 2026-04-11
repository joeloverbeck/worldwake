# Simulation Remediation Proposals

**Status**: ✅ COMPLETED

Source report: `reports/simulation-observer-report.md`
Generated: 2026-04-10

## Context

The simulation ran 4 agents (Kael, Merchant Vara, Forager Lina, Guard Theron) across 5 places for 1440 ticks (1 simulated day) using scenario `scenarios/cli-evaluation.ron` with seed 7777. The dominant failure mode is **budget-exhaustion-driven behavioral collapse**: the GOAP planner cannot find multi-step travel+acquire plans within its 300-expansion budget, trapping agents at resource-poor locations in sleep+relieve loops. Guard Theron died at tick 422 from hunger deprivation. Forager Lina became permanently stuck at tick ~732 due to inventory saturation with Waste. Zero trade, zero crafting, and zero market activity occurred across the entire simulation. 3 of 8 findings are CRITICAL, 2 HIGH, 3 MEDIUM.

The root cause chain is: (1) planner budget too low for branchy multi-location plans -> (2) agents cannot acquire food/water at remote locations -> (3) critical needs escalate unchecked -> (4) survival goals dominate goal ranking but remain unsatisfiable -> (5) all non-survival goals (social, economic, patrol) are crowded out -> (6) behavioral collapse to sleep+relieve loops -> (7) death or permanent stall. A secondary chain affects Forager Lina: consumption produces Waste -> no disposal mechanism -> inventory saturates -> cannot pick up new resources -> permanent idle.

## Proposed Golden Tests

### GT-1: Behavioral Diversity Under Remote Resource Scarcity (Extended)
**Source finding**: Finding 2 (Action Loops) + Finding 5 (Sustained Critical Needs)
**Severity**: HIGH
**File**: `crates/worldwake-ai/tests/golden_simulation_gaps.rs`
**Setup**: 2 agents at a barren location (no food/water affordances). A resource-rich location exists 1 hop away with food and water. Agents have standard cognitive profiles. Run for 500 ticks.
**Assertion**: No agent performs the same 2-action sequence (e.g., sleep->relieve->sleep->relieve) for more than 150 consecutive ticks. At least one agent attempts travel toward the resource location within 300 ticks.
**Rationale**: Existing `golden_max_idle_under_remote_resource_scarcity` tests that agents don't idle for 100+ ticks, but doesn't test for degenerate action *loops* where the agent is technically active but cycling between only 2 actions. This test ensures behavioral diversity even under scarcity pressure.
**Existing coverage**: `golden_simulation_gaps.rs::golden_max_idle_under_remote_resource_scarcity` covers idle bounds but not action-loop detection. `golden_ai_decisions.rs::golden_fallback_to_addressable_need_when_top_need_unsatisfiable` covers fallback to addressable needs but only for 200 ticks.

### GT-2: Waste Accumulation Does Not Permanently Stall Agent
**Source finding**: Finding 3 (Stuck Agents -- Forager Lina)
**Severity**: HIGH
**File**: `crates/worldwake-ai/tests/golden_simulation_gaps.rs`
**Setup**: 1 agent at a location with a harvestable resource (e.g., OrchardRow). Agent has carry capacity of 10. Agent starts with 8 Waste items in inventory. Food source available locally. Run for 300 ticks.
**Assertion**: Agent does not idle for more than 100 consecutive ticks. Agent either: (a) successfully drops/discards waste to free capacity, or (b) the planner generates a FreeCarryCapacity goal that leads to a committed action.
**Rationale**: The simulation showed Forager Lina permanently stuck with 12 Waste items and no way to clear them. Existing golden tests cover waste *creation* at latrines but not waste *accumulation blocking agent behavior*. This test protects the invariant that inventory saturation must not be a permanent dead end.
**Existing coverage**: `golden_ai_decisions.rs` tests latrine waste creation. `golden_perception_exposure.rs` tests waste entity memory eviction. Neither tests waste-induced behavioral stall.

### GT-3: Minimal Economic Activity in Multi-Agent Scenario
**Source finding**: Finding 10 (Economic Stagnation)
**Severity**: CRITICAL
**File**: `crates/worldwake-ai/tests/golden_emergent.rs`
**Setup**: 3 agents (merchant, forager, consumer) across 2 locations. One location has harvestable resources, the other has a market stall and consumer demand. Agents have appropriate role profiles. Run for 1000 ticks.
**Assertion**: At least 1 trade action commits within 1000 ticks. At least 1 harvest action commits. The merchant attempts staff_market at least once with a committed outcome (not all StartFailed).
**Rationale**: The simulation showed zero trade and zero crafting across 1440 ticks with 4 agents. Existing `golden_supply_chain.rs` and `golden_trade.rs` test trade mechanics in curated setups, but no golden test asserts that a multi-agent scenario with appropriate roles produces *any* economic activity within a reasonable timeframe. This is a regression guard against complete economic failure.
**Existing coverage**: `golden_supply_chain.rs` tests specific merchant restock chains. `golden_trade.rs` tests negotiation mechanics. Neither tests emergent economic activity from role-diverse agents.

## Proposed Spec Changes

### SC-1: Belief Lifecycle -- Formation, Retention, and Decay of Place-Knowledge Beliefs
**Source finding**: Finding 8 (Belief Staleness) + TQ-2
**Severity**: MEDIUM
**Spec**: New spec recommended (no existing spec covers belief lifecycle comprehensively)
**Section**: N/A (new spec)
**Change**: Specify how agents form, retain, and lose beliefs about places they have visited and resources they have observed there. The spec should define:
  - **Place-knowledge belief formation**: When an agent visits a location and perceives resources (Well, OrchardRow, etc.), a belief about "resource X exists at place Y" should be formed and retained.
  - **Belief retention rules**: Place-knowledge beliefs should persist beyond the agent's physical departure from the location, with configurable retention duration per agent profile (memory fidelity parameter).
  - **Belief decay**: Define when and how place-knowledge beliefs degrade -- time-based decay, memory capacity eviction, or both. The decay mechanism must be a per-agent profile parameter (FND-22: Agent Diversity).
  - **Planner integration**: The planner must be able to use retained place-knowledge beliefs to generate travel+acquire plans (e.g., "I believe there is water at Thornwall Village, so I can plan: travel to Thornwall -> drink").
**FOUNDATIONS alignment**: FND-14 (World State Is Not Belief State), FND-15 (Knowledge Is Acquired Locally), FND-16 (Ignorance and Uncertainty Are First-Class), FND-20 (Resource-Bounded Practical Reasoning), FND-22 (Agent Diversity Through Concrete Variation).

## Proposed Tickets

### TK-1: Increase Planner Expansion Budget or Improve Candidate Pruning for Multi-Location Plans
**Source finding**: Finding 5 (Sustained Critical Needs) + Cross-Cutting Pattern 1 (Budget-Exhaustion-Driven Behavioral Collapse)
**Priority**: P0
**Crate(s)**: `worldwake-ai`
**Description**: The planner's 300-expansion budget is insufficient for travel+acquire plans. AcquireCommodity goals consistently budget-exhaust with 1400-2600 candidates at each step, branching too widely for the budget to find a viable plan. This is the root cause of behavioral collapse for 3 of 4 agents. Options to investigate:
  1. Increase the default expansion budget (risk: slower planning per tick)
  2. Improve candidate pruning/heuristic guidance to reduce branching factor (preferred -- reduces search space without increasing budget)
  3. Add travel-aware goal decomposition so the planner doesn't explore all possible acquisition paths at every location
**Dependencies**: None (root cause -- all other findings depend on this)
**FOUNDATIONS alignment**: FND-20 (Resource-Bounded Practical Reasoning -- agents must reason as limited actors, but their limits should not prevent *any* viable plan from being found when one clearly exists), FND-8 (Every Action Has Preconditions -- travel+acquire is the lawful path, and it must be reachable by the planner)
**Acceptance criteria**: In the cli-evaluation scenario (seed 7777), at least 2 agents successfully plan and execute a travel+acquire sequence for food or water within 500 ticks. AcquireCommodity goals no longer budget-exhaust for single-hop travel plans.

### TK-2: Implement Waste Disposal Action (Drop or Discard)
**Source finding**: Finding 3 (Stuck Agents -- Forager Lina) + Cross-Cutting Pattern 4 (Waste Accumulation Cascade)
**Priority**: P1
**Crate(s)**: `worldwake-systems`, `worldwake-core`
**Description**: Agents have no mechanism to discard Waste items from their inventory. Consumption produces Waste, but without a disposal action, inventory eventually saturates and the agent can no longer pick up resources. Forager Lina accumulated 12 Waste items and became permanently idle at tick ~732. The FreeCarryCapacity goal exists but has no supporting action to clear waste. Implement a `discard` or `drop` action that allows agents to remove Waste from inventory and place it at the current location (or destroy it if appropriate). The action should have duration and preconditions consistent with FND-8.
**Dependencies**: None
**FOUNDATIONS alignment**: FND-4 (Persistent Identity and Explicit Transfer -- waste must be explicitly placed somewhere, not silently destroyed), FND-8 (Every Action Has Preconditions), FND-11 (Every Positive Feedback Loop Needs a Physical Dampener -- waste accumulation is an undampened positive feedback loop: more consumption -> more waste -> less capacity -> less consumption capability)
**Acceptance criteria**: An agent with full inventory of Waste can plan and execute a discard action that removes at least 1 Waste item and frees carry capacity. The discarded waste appears at the agent's current location (conservation invariant).

### TK-3: Fix Guard Role Goal Generation to Include Survival Needs
**Source finding**: Finding 6 (Unaddressed Needs -- Guard Theron)
**Priority**: P1
**Crate(s)**: `worldwake-ai`
**Description**: Guard Theron never generated AcquireCommodity goals for food or water. His goal list contained only InvestigateViolation, Patrol, Relieve, ShareBelief, and Sleep. His guard role profile's goal generator either doesn't produce survival goals or ranks them below duty goals permanently. With hunger averaging 915 and thirst 943, survival goals should have been generated and ranked highly. Investigate why the goal generator skips food/water for guard-profiled agents and ensure survival needs generate goals for all agent profiles.
**Dependencies**: None
**FOUNDATIONS alignment**: FND-19 (Agent Symmetry -- all agents use the same needs system; guard role should not exempt an agent from survival goal generation), FND-22 (Agent Diversity -- role affects *priorities*, not *whether survival goals exist at all*)
**Acceptance criteria**: A guard-profiled agent with hunger > 750 generates at least one AcquireCommodity{Food} or Eat goal within 50 ticks. Guard duty goals may still be ranked higher at lower need levels, but survival goals must appear when needs are critical.

### TK-4: Fix staff_market Precondition at Dusty Trail
**Source finding**: Finding 4 (Failed Action Spirals -- Merchant Vara)
**Priority**: P2
**Crate(s)**: `worldwake-systems` or `worldwake-cli` (scenario configuration)
**Description**: Merchant Vara attempted staff_market 5 times at Dusty Trail, all StartFailed. The specific precondition failure is opaque (linked to TQ-4). Either: (a) Dusty Trail needs a MarketStall component for staff_market to succeed (scenario configuration issue), or (b) the planner should not generate SellCommodity goals at locations without market infrastructure (candidate generation issue). Investigate which precondition fails and either fix the scenario or improve the planner's feasibility check so it doesn't generate Unlikely plans.
**Dependencies**: Blocked by TK-7 (TQ-4 -- knowing the specific failed precondition would clarify the fix)
**FOUNDATIONS alignment**: FND-8 (Every Action Has Preconditions -- preconditions must be transparent and checkable), FND-29 (Debuggability -- StartFailed without a reason is an introspection gap)
**Acceptance criteria**: In a scenario where a merchant is at a location with no market stall, either: (a) staff_market is not attempted (planner filters it), or (b) the precondition failure reason is logged clearly.

### TK-5: Reduce Redundant Self-Observation in Perception System
**Source finding**: Finding 1 (Redundant Perception)
**Priority**: P2
**Crate(s)**: `worldwake-systems`
**Description**: Agents observe themselves every perception tick (Kael: 112 times, Guard Theron: 112 times). Self-observation is wasteful -- an agent's own state is already available through introspection/need system without requiring a perception event. Additionally, entities with unchanged state are re-observed on every perception tick. Consider: (a) skipping self as a perception target, (b) implementing a "state-changed-since-last-observation" filter to avoid redundant observations of unchanged entities.
**Dependencies**: None
**FOUNDATIONS alignment**: FND-12 (Performance May Compress Computation, Never Causality -- reducing redundant perception is a computation optimization that doesn't change causal outcomes, since the agent already has access to its own state)
**Acceptance criteria**: Self-observation events are either eliminated or reduced by >80%. No behavioral regression in existing golden tests.

### TK-6: Belief History in Observer Binary (Trace Quality)
**Source finding**: TQ-2 (Belief summary doesn't distinguish "never formed" vs. "formed and decayed")
**Priority**: P1
**Crate(s)**: `worldwake-cli` (observer binary)
**Description**: The observer report could not determine whether agents forgot about Thornwall Village resources or never formed those beliefs. This directly impacted root-cause analysis for Finding 8 (Belief Staleness) and Finding 5 (why agents don't travel to resource locations). Add a "belief history" subsection to the observer dump that aggregates belief formation and decay events from the event log (e.g., "tick 5: formed belief about Well at Thornwall Village", "tick 200: belief decayed/evicted"). This does not require new events -- it aggregates existing BeliefUpdate events.
**Dependencies**: None
**FOUNDATIONS alignment**: FND-29 (Debuggability Is a Product Feature -- "Why did this agent not know about the Well?" must be answerable)
**Acceptance criteria**: Observer dump includes per-agent belief timeline showing formation and loss of place-knowledge and entity-knowledge beliefs.

### TK-7: Include Failed Precondition in StartFailed Reporting
**Source finding**: TQ-4 (staff_market StartFailed doesn't include the specific precondition that failed)
**Priority**: P2
**Crate(s)**: `worldwake-cli` (observer binary), possibly `worldwake-sim` (action framework)
**Description**: When an action fails to start, the observer dump shows "StartFailed" but not which precondition was unmet. This made it impossible to diagnose Merchant Vara's staff_market failures (Finding 4) and reduced confidence in root-cause analysis. Include the specific failed precondition name or description in the observer dump's action reporting.
**Dependencies**: None
**FOUNDATIONS alignment**: FND-29 (Debuggability), FND-8 (Every Action Has Preconditions -- precondition failures should be transparent)
**Acceptance criteria**: Observer dump shows the specific precondition that failed for each StartFailed action (e.g., "staff_market StartFailed: NoMarketStallAtLocation").

### TK-8: Inventory Capacity Timeline in Observer Binary (Trace Quality)
**Source finding**: TQ-3 (No waste/inventory capacity tracking over time)
**Priority**: P2
**Crate(s)**: `worldwake-cli` (observer binary)
**Description**: The observer report could only see end-state inventory, not when Forager Lina's capacity was reached. Add carry capacity utilization tracking per agent in 100-tick bins (e.g., "0-99: 2/10 slots, 100-199: 5/10 slots") to the observer dump. This is computed from inventory change events already in the event log.
**Dependencies**: None
**FOUNDATIONS alignment**: FND-29 (Debuggability)
**Acceptance criteria**: Observer dump includes per-agent inventory capacity utilization over time in binned format.

## Findings Deferred or Not Requiring Independent Remediation

| Finding | Severity | Reason for Deferral |
|---------|----------|---------------------|
| Finding 7 (Impossible Knowledge) | NONE | Clean -- no evidence of agents acting on unperceived information. |
| Finding 9 (Social Isolation) | MEDIUM | Downstream symptom of Finding 5. Social actions collapse when survival needs dominate goal ranking but remain unsatisfiable. Once TK-1 enables viable survival plans, social goals should re-emerge in planning headroom. |
| Finding 6 -- Merchant Vara's thirst | CRITICAL | Downstream symptom of Finding 5. Vara's AcquireCommodity{Water} goals budget-exhaust. Deferred to TK-1 (planner budget). Guard Theron's missing goals are addressed independently in TK-3. |
| Finding 2 (Action Loops) -- Kael, Vara | HIGH | Downstream symptom of Finding 5. Sleep+relieve loops result from planner inability to find survival plans. GT-1 adds a regression guard, but the fix is TK-1. |
| Finding 10 (Economic Stagnation) -- trade/craft | CRITICAL | Downstream of Findings 3 (waste blocks production), 4 (market failure), and 5 (agents can't travel to production sites). GT-3 adds a regression guard. Root fixes are TK-1, TK-2, TK-4. |
| TQ-1 (No per-agent goal generation log) | N/A | Acceptable trade-off per observer report. Would increase dump size significantly. Current affordance + blocked desires data is sufficient for diagnosis. |

## Summary

| Type | Count | Severity Breakdown |
|------|-------|--------------------|
| Golden Tests | 3 | 1 CRITICAL, 2 HIGH |
| Spec Changes | 1 | 1 MEDIUM |
| Tickets | 8 | 1 P0, 3 P1, 4 P2 |
| Deferred | 6 | 2 CRITICAL (downstream), 1 HIGH, 1 MEDIUM, 1 NONE, 1 N/A |

### Root Cause Chain

```
TK-1 (Planner budget/pruning) [P0, ROOT CAUSE]
  |-- Finding 5 (Sustained Critical Needs) -- all agents
  |-- Finding 6 (Unaddressed Needs -- Vara's thirst) -- deferred
  |-- Finding 2 (Action Loops) -- GT-1 regression guard
  |-- Finding 9 (Social Isolation) -- deferred
  +-- Finding 10 (Economic Stagnation) -- GT-3 regression guard
        |-- TK-4 (staff_market precondition) [P2]
        |     +-- TK-7 (StartFailed reporting) [P2, unblocks diagnosis]
        +-- TK-2 (Waste disposal action) [P1]
              +-- Finding 3 (Stuck Agents -- Lina) -- GT-2 regression guard

TK-3 (Guard goal generation) [P1, INDEPENDENT]
  +-- Finding 6 (Unaddressed Needs -- Theron's missing food/water goals)

SC-1 (Belief lifecycle spec) [INDEPENDENT]
  +-- Finding 8 (Belief Staleness)
        +-- TK-6 (Belief history in observer) [P1, unblocks diagnosis]

TK-5 (Redundant perception) [P2, INDEPENDENT]
  +-- Finding 1 (Redundant Perception)

TK-8 (Inventory capacity timeline) [P2, INDEPENDENT]
```

**Recommended implementation order:**
1. **First**: TK-1 (planner budget -- root cause, unblocks most findings)
2. **Parallel with TK-1**: TK-2 (waste disposal), TK-3 (guard goals), TK-6 (belief history), TK-7 (StartFailed reporting)
3. **After TK-1**: TK-4 (staff_market -- TK-7 helps diagnose), GT-1, GT-2, GT-3
4. **Independent/lower priority**: TK-5 (redundant perception), TK-8 (inventory timeline), SC-1 (belief lifecycle spec)

### FOUNDATIONS Alignment

| Principle | Status | Notes |
|-----------|--------|-------|
| FND-1 (Maximal Emergence) | VIOLATED | Behavioral collapse produces homogeneous sleep-loop behavior instead of emergent diversity. TK-1 addresses root cause. |
| FND-4 (Persistent Identity, Explicit Transfer) | ENFORCED | Waste items persist correctly. TK-2 proposes explicit disposal action (not silent destruction). |
| FND-8 (Every Action Has Preconditions) | ENFORCED but OPAQUE | Actions have preconditions, but failures don't report which precondition failed. TK-7 addresses. |
| FND-11 (Positive Feedback Dampener) | VIOLATED | Waste accumulation is an undampened positive feedback loop (consumption -> waste -> less capacity -> less consumption). TK-2 provides the dampener (disposal action). |
| FND-12 (Performance May Compress, Never Causality) | APPLICABLE | TK-5 (perception optimization) is a valid computation compression. |
| FND-14 (World State Is Not Belief State) | ENFORCED | Agents plan from beliefs. But belief formation gaps mean agents lack the knowledge to plan effectively. SC-1 addresses. |
| FND-15 (Knowledge Acquired Locally) | ENFORCED | No impossible knowledge detected (Finding 7 clean). |
| FND-16 (Ignorance Is First-Class) | ENFORCED but GAPS | Agents correctly don't know about remote resources. The problem is they *should* have formed beliefs during earlier visits but apparently didn't. SC-1 + TK-6 investigate. |
| FND-19 (Agent Symmetry) | VIOLATED | Guard role suppresses survival goal generation, creating an asymmetry where guards cannot plan to eat/drink. TK-3 addresses. |
| FND-20 (Resource-Bounded Reasoning) | VIOLATED | Planner budget prevents *any* multi-location plan from being found, making agents unreasonably limited. TK-1 addresses. |
| FND-22 (Agent Diversity) | VIOLATED | All agents at Dusty Trail converge on identical sleep+relieve behavior. TK-1 addresses root cause. |
| FND-29 (Debuggability) | GAPS | StartFailed reasons opaque (TK-7), belief history missing (TK-6), inventory timeline missing (TK-8). |

## Outcome

- **Completion date**: 2026-04-10
- **What changed**: Proposed 3 golden tests (GT-1 through GT-3), 1 spec change (SC-1: belief lifecycle), and 8 tickets (TK-1 through TK-8) derived from the observer report's 8 findings. Established root-cause dependency chain with TK-1 (planner budget) as the P0 root cause.
- **Deviations**: None — remediation followed the simulation-remediation skill methodology.
- **Verification**: Proposals have been exploited in subsequent implementation work addressing planner budget, waste disposal, guard goal generation, and observer enhancements.
