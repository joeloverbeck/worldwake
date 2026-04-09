# S79: Resource-Source Consumption Affordances

## Summary

Close the gap between resource sources at a location and agent consumption. Currently, agents at places with resource sources (e.g., Water at a Well, Apples at an OrchardRow) cannot plan or execute the harvest-then-consume chain reliably. The eat/drink actions correctly require possession (`TargetSpec::EntityDirectlyPossessedByActor`), and harvest actions correctly target co-located facilities with resource sources, but the connection fails in practice: either agents lack recipe knowledge for basic harvests, the planner cannot chain AcquireCommodity through harvest within its expansion budget, or candidate generation does not compose the multi-step chain correctly. This spec ensures that an agent at a location with a resource source can plan and execute the full harvest-to-consume chain within a single planning cycle.

## Phase

Phase 7: Consequence Carriers (Adjunct — Simulation Remediation)

## Status

Draft

## Crates

- `worldwake-systems` (needs actions, production actions, affordance registration)
- `worldwake-sim` (affordance query, action semantics)
- `worldwake-ai` (candidate generation, goal model, search)
- `worldwake-core` (recipe registry, production types)

## Dependencies

- E10 (production/transport) — completed
- E06 (needs/metabolism) — completed
- S01 (production output ownership) — completed

## Design Goals

- An agent at a place with a harvestable resource source, who knows the harvest recipe, can plan and execute: harvest → eat/drink within a single AcquireCommodity planning cycle
- The planning chain does not require a separate AcquireCommodity goal followed by a separate ConsumeOwnedCommodity goal — the planner finds the full chain in one search
- No changes to eat/drink action semantics — possession requirement stays (FND-04)
- No changes to harvest action semantics — recipe knowledge and co-location stay (FND-08)
- Root-cause diagnosis: identify which link in the harvest-to-consume chain breaks and fix that specific link

## Non-Goals

- Making eat/drink work without possession (violates FND-04 explicit transfer)
- Removing recipe knowledge requirements from harvest (violates FND-08 preconditions)
- General plan search budget restructuring — that is a separate concern (CognitiveProfile already supports per-agent tuning)
- Adding new commodity types or resource source variants
- Exploration or geographic knowledge acquisition (deferred to S80)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P1 (Maximal Emergence) | Agents that can harvest and consume create downstream economic activity — trade, competition, surplus, social interaction |
| P4 (Persistent Identity / Explicit Transfer) | Harvest explicitly transfers commodity from ResourceSource to agent inventory; eat/drink explicitly consumes from inventory. No shortcut. |
| P5 (Carriers of Consequence) | Working consumption chains are prerequisites for all downstream economic emergence |
| P8 (Preconditions, Duration, Cost) | Harvest has recipe knowledge, co-location, tool requirements. Eat/drink has possession. Both preserved. |
| P20 (Resource-Bounded Reasoning) | The fix must work within existing CognitiveProfile budgets, not by inflating the budget |
| P26 (Systems Interact Through State) | Harvest writes to inventory state; eat/drink reads inventory state. No direct cross-system coupling. |
| P28 (No Backward Compat) | If the fix changes how candidate generation composes multi-step chains, old path is removed, not wrapped |

## Section H: Causal Hooks

### H1. Information-Path Analysis

The harvest-to-consume chain requires:
1. Agent perceives Facility with ResourceSource at their location (perception system → belief about resource source)
2. Agent has recipe knowledge for the corresponding harvest recipe (RecipeKnowledge component)
3. Candidate generation for AcquireCommodity considers harvest as a terminal action or intermediate step
4. Planner chains: harvest (effect: agent gains commodity) → eat/drink (precondition: agent possesses commodity)

Information enters through perception (step 1) and initial agent configuration (step 2). No global queries.

### H2. Positive-Feedback Analysis

No new positive-feedback loops. Harvest depletes ResourceSource.available_quantity; regeneration is already rate-limited by `regeneration_ticks_per_unit`.

### H3. Concrete Dampeners

ResourceSource depletion: `available_quantity` decreases with each harvest. Regeneration is bounded by `regeneration_ticks_per_unit`. Competition: multiple agents harvesting the same source deplete it faster (contention through reservation system).

### H4. Stored State vs. Derived

- **Stored**: ResourceSource.available_quantity, agent inventory (Inventory component), RecipeKnowledge component
- **Derived**: affordance set (recomputed each planning cycle), candidate list (recomputed each goal evaluation)

## Deliverables

### 1. Root-Cause Diagnosis

Investigate and document which link in the chain fails. The candidates are:

**A. Recipe knowledge gap**: Agents in scenarios may lack `RecipeKnowledge` entries for basic harvest recipes (e.g., `harvest:Harvest Apples`, `harvest:Harvest Water`). If so, fix by ensuring scenario `AgentDef` can specify recipe knowledge, and default agents at locations with resource sources get the corresponding harvest recipes.

**B. Candidate generation gap**: `generate_candidates` for `AcquireCommodity { commodity }` may not consider harvest actions as a path to acquiring the commodity. The candidate generator should recognize that a harvest action at a co-located facility produces the desired commodity.

**C. Planner effect modeling gap**: The planner's `PlannerEffect` for harvest may not correctly model "agent gains commodity X" as a world-state change that enables subsequent eat/drink. If the planner does not track inventory-gain effects from harvest, it cannot chain harvest → eat.

**D. Budget exhaustion**: Even with correct modeling, the search space may branch too widely. If this is the sole cause, the fix should be candidate pruning (belief-informed filtering of irrelevant locations/recipes), not budget inflation.

The diagnosis must identify which of A/B/C/D applies (possibly multiple) and the fix must address the actual root cause.

### 2. Fix Implementation

Based on the diagnosis:

- **If A**: Add `RecipeKnowledge` population to scenario spawning for agents at resource source locations. Ensure `AgentDef` supports a `known_recipes: Vec<String>` field (or similar) in the RON scenario definition. Universal recipe knowledge for basic survival harvests (water, food) should be a default for all agents.

- **If B**: Extend `generate_candidates` in `crates/worldwake-ai/src/candidate_generation.rs` to include harvest actions when evaluating `AcquireCommodity` goals. The generator should check: does the agent's current place have a Facility with a ResourceSource for the desired commodity? If so, include the corresponding harvest action as a candidate.

- **If C**: Extend planner effect declarations for harvest actions to include `PlannerEffect::AgentGainsCommodity { commodity, quantity }` (or the existing effect representation). This enables the search to chain harvest → eat/drink.

- **If D**: Add belief-informed candidate pruning: when generating AcquireCommodity candidates, filter harvest candidates to only those at places the agent believes have the relevant resource source. This reduces branching without changing the budget.

### 3. Verification

After the fix, the following must hold:

- An agent at a place with a Water resource source (on a Facility the agent can access) and the corresponding harvest recipe knowledge can plan and execute: harvest water → drink, within a single planning cycle
- An agent at a place with an Apple resource source (on an OrchardRow) can plan and execute: harvest apples → eat
- The plan search completes within the default CognitiveProfile budget (224 expansions)
- Existing golden tests continue to pass

## SystemFn Integration

No new SystemFn. The fix operates within existing systems:
- Affordance generation (existing tick order)
- Candidate generation (existing planning pipeline)
- Plan search (existing GOAP search)

## Component Registration

Potential new components (depending on root cause):
- If A: `RecipeKnowledge` may need scenario-definable configuration in `AgentDef`. Check if `RecipeKnowledge` is already in `AgentDef` — if not, add it following the profile scenario contract (spec-drafting-rules section 5).

No new authoritative components expected — this is primarily a fix to the planning/affordance pipeline.

## Cross-System Interactions

- **Production system → Needs system**: Harvest writes commodity to agent inventory (state-mediated). Eat/drink reads from inventory (state-mediated). No direct coupling.
- **Perception system → Planning**: Agent must perceive the resource source to form beliefs about it. Planning reads beliefs. Existing flow, no changes.
- **Candidate generation → Search**: Candidates are generated, search finds plans. Existing flow, fix may change candidate composition.
