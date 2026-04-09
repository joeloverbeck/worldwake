# S79: Resource-Source Consumption Affordances

## Summary

Close the gap between resource sources at a location and agent consumption. Currently, agents at places with resource sources (e.g., Water at a Well, Apples at an OrchardRow) cannot plan or execute the harvest-then-consume chain reliably. The eat/drink actions correctly require possession (`TargetSpec::EntityDirectlyPossessedByActor`), and harvest actions correctly target co-located facilities with resource sources, but the connection fails in practice due to two confirmed root causes: (1) agents spawned from scenarios lack `KnownRecipes` entries for harvest recipes because the component is not wired into `AgentDef` or `spawn_agent()`, and (2) the planner's `apply_planner_step()` treats `PlannerOpKind::Harvest` as a no-op, so it cannot predict commodity gain from harvest and therefore cannot chain harvest → consume. This spec fixes both root causes so that an agent at a location with a resource source can plan and execute the full harvest-to-consume chain.

## Phase

Phase 7: Consequence Carriers (Adjunct — Simulation Remediation)

## Status

Draft

## Crates

- `worldwake-systems` (needs actions, production actions, affordance registration)
- `worldwake-sim` (affordance query, action semantics)
- `worldwake-ai` (candidate generation, goal model, search)
- `worldwake-core` (recipe registry, production types)
- `worldwake-cli` (scenario spawning — `AgentDef`, `spawn_agent()`)

## Dependencies

- E10 (production/transport) — completed (`archive/specs/E10-production-transport.md`)
- E09 (needs/metabolism) — completed (`archive/specs/E09-needs-metabolism.md`)
- S01 (production output ownership) — completed (`archive/specs/S01-production-output-ownership-claims.md`)

## Design Goals

- An agent at a place with a harvestable resource source, who knows the harvest recipe, can plan and execute: `AcquireCommodity(SelfConsume)` (harvest) followed by `ConsumeOwnedCommodity` (eat/drink), with the planner correctly predicting that harvest yields the commodity needed for consumption
- The planner's hypothetical state tracks commodity gain from harvest, enabling the `AcquireCommodity` goal's satisfaction check (`commodity_quantity(actor, commodity) > 0`) to succeed after a Harvest step
- Agents spawned from scenarios can receive recipe knowledge via `AgentDef`, enabling harvest actions whose precondition `Constraint::ActorKnowsRecipe(RecipeId)` checks the agent's `KnownRecipes` component
- No changes to eat/drink action semantics — possession requirement stays (FND-04)
- No changes to harvest action semantics — recipe knowledge and co-location stay (FND-08)

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
| P8 (Preconditions, Duration, Cost) | Harvest has recipe knowledge (`Constraint::ActorKnowsRecipe`), co-location, tool requirements. Eat/drink has possession (`TargetDirectlyPossessedByActor`). Both preserved. |
| P20 (Resource-Bounded Reasoning) | The fix must work within existing CognitiveProfile budgets, not by inflating the budget |
| P26 (Systems Interact Through State) | Harvest writes to inventory state; eat/drink reads inventory state. No direct cross-system coupling. |
| P28 (No Backward Compat) | If the fix changes how candidate generation composes multi-step chains, old path is removed, not wrapped |

## Section H: Causal Hooks

### H1. Information-Path Analysis

The harvest-to-consume chain requires:
1. Agent perceives Facility with ResourceSource at their location (perception system → belief about resource source)
2. Agent has recipe knowledge for the corresponding harvest recipe (`KnownRecipes` component containing the harvest `RecipeId`)
3. Candidate generation for `AcquireCommodity(SelfConsume)` considers harvest as an operation via `ACQUIRE_OPS` (which includes `PlannerOpKind::Harvest`)
4. Planner chains: harvest (effect: agent gains commodity via `with_commodity_quantity`) → eat/drink (precondition: agent possesses commodity)

Information enters through perception (step 1) and initial agent configuration (step 2). No global queries.

### H2. Positive-Feedback Analysis

No new positive-feedback loops. Harvest depletes ResourceSource.available_quantity; regeneration is already rate-limited by `regeneration_ticks_per_unit`.

### H3. Concrete Dampeners

ResourceSource depletion: `available_quantity` decreases with each harvest. Regeneration is bounded by `regeneration_ticks_per_unit`. Competition: multiple agents harvesting the same source deplete it faster (contention through reservation system).

### H4. Stored State vs. Derived

- **Stored**: ResourceSource.available_quantity, agent possession graph (ItemLot entities with DirectlyPossessedBy relations), `KnownRecipes` component (`BTreeSet<RecipeId>`)
- **Derived**: affordance set (recomputed each planning cycle), candidate list (recomputed each goal evaluation)

## Deliverables

### 1. Wire `KnownRecipes` into Scenario Spawning

**Confirmed root cause A**: The `KnownRecipes` component exists in `worldwake-core` (`crates/worldwake-core/src/production.rs:38-57`) and is registered on `EntityKind::Agent` in `component_schema.rs`, but it is not present in `AgentDef` (`crates/worldwake-cli/src/scenario/types.rs`) and `spawn_agent()` (`crates/worldwake-cli/src/scenario/mod.rs`) never calls `set_component_known_recipes()`. Agents spawned from scenarios therefore have empty `KnownRecipes` and cannot pass the `Constraint::ActorKnowsRecipe(RecipeId)` precondition on harvest actions.

**Fix**:
- Add `known_recipes: Option<Vec<String>>` field to `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs`. Recipe names are resolved to `RecipeId` values via the `RecipeRegistry` at spawn time.
- Add a conditional `if let Some(recipe_names) = &agent_def.known_recipes { ... }` block in `spawn_agent()` that resolves names, constructs `KnownRecipes::with(resolved_ids)`, and calls `txn.set_component_known_recipes(agent_id, known_recipes)`.
- **Profile classification**: `KnownRecipes` is **role-specific** per spec-drafting-rules section 5 — not every agent needs recipes to function as a reasoning agent. Conditional application, no mandatory Default.

### 2. Add Harvest Effect to Planner Hypothetical State

**Confirmed root cause C**: In `crates/worldwake-ai/src/goal_model.rs` (lines 1034-1056), `PlannerOpKind::Harvest` falls through to the identity arm (`=> state`), meaning the planner never models "agent gains commodity" from harvest. The `AcquireCommodity` satisfaction check (`commodity_quantity(actor, commodity) > Quantity(0)` at line 1147) can never be predicted as satisfied after a Harvest step, breaking the planner's ability to chain harvest → consume.

**Fix**:
- Add a match arm for `PlannerOpKind::Harvest` in `apply_planner_step()` that extracts `output_commodity` and `output_quantity` from the `HarvestActionPayload` (defined in `crates/worldwake-sim/src/action_payload.rs:321-327`) and calls `state.with_commodity_quantity(actor, output_commodity, current_qty + output_quantity)`.
- This follows the same pattern used for `PlannerOpKind::Loot` and `PlannerOpKind::Bribe` which already mutate hypothetical commodity quantities.

### 3. Verification

After the fix, the following must hold:

- An agent at a place with a Water resource source (on a Facility the agent can access) and the corresponding harvest recipe in their `KnownRecipes` can plan and execute: harvest water → drink, with the planner predicting commodity gain from the harvest step
- An agent at a place with an Apple resource source (on an OrchardRow) can plan and execute: harvest apples → eat
- The plan search completes within the default CognitiveProfile budget (224 expansions, `max_node_expansions` field)
- Existing golden tests continue to pass

## SystemFn Integration

No new SystemFn. The fix operates within existing systems:
- Affordance generation (existing tick order)
- Candidate generation (existing planning pipeline)
- Plan search (existing GOAP search — `apply_planner_step()` modification)

## Component Registration

`KnownRecipes` is already registered on `EntityKind::Agent` in `component_schema.rs`. This spec adds only the scenario wiring:
- New field in `AgentDef` (`known_recipes: Option<Vec<String>>`)
- New `set_component_known_recipes()` call in `spawn_agent()`

Classification: **role-specific** (conditional application per spec-drafting-rules section 5).

## Cross-System Interactions

- **Production system → Needs system**: Harvest writes commodity to agent possession graph (state-mediated). Eat/drink reads from possession graph (state-mediated). No direct coupling.
- **Perception system → Planning**: Agent must perceive the resource source to form beliefs about it. Planning reads beliefs. Existing flow, no changes.
- **Candidate generation → Search**: Candidates are generated with `ACQUIRE_OPS` (including Harvest). Search uses `apply_planner_step()` to predict effects. Fix changes Harvest's hypothetical state mutation.
