# S79RESSOUCON-003: Golden test — harvest-to-consume chain

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S79RESSOUCON-001, S79RESSOUCON-002, S79RESSOUCON-004

## Problem

There is no golden E2E test that verifies an agent can plan and execute the full harvest → eat/drink chain end-to-end. The focused unit tests in tickets 001 and 002 prove the individual fixes work, but only a golden test proves the entire pipeline integrates: candidate generation emits `AcquireCommodity(SelfConsume)`, the planner chains Harvest → Consume with correct hypothetical state, and the authoritative action execution actually transfers the commodity and satisfies the need.

## Assumption Reassessment (2026-04-09)

1. Existing golden test infrastructure in `crates/worldwake-ai/tests/`: `golden_production.rs` tests harvest actions with `KnownRecipes::with(...)` in Rust harness code. `golden_supply_chain.rs` tests merchant harvest → carry → craft → sell chains. `conformance_execution_budget.rs` has `setup_local_consume_harness()` for immediate local consume. None test the scenario-spawned harvest → eat/drink chain specifically.
2. `GoldenHarness` (in `crates/worldwake-ai/tests/golden_harness/mod.rs`) provides `with_recipes()`, `agent_commodity_qty()`, and recipe lookup by name. Agents need `PerceptionProfile` to observe newly created entities from production.
3. Shared boundary: golden E2E harness → full tick simulation → candidate generation + planner + action execution. This test exercises the complete pipeline from scenario setup to need satisfaction.
4. This is a golden E2E ticket. The live `GoalKind` under test is `AcquireCommodity { commodity, purpose: SelfConsume }` leading to `ConsumeOwnedCommodity`. The planner uses `ACQUIRE_OPS` (including `PlannerOpKind::Harvest`) and `CONSUME_OPS` (including `PlannerOpKind::Consume`).
5. Budget: default `CognitiveProfile.max_node_expansions` is 224. The test must verify the plan completes within this budget.
6. Ticket 001 completed the scenario/bootstrap path only for the currently canonical production recipes (`Harvest Apples`, `Harvest Grain`, `Bake Bread`). The lawful water harvest contract remains uncovered and is now owned by `S79RESSOUCON-004`. Correction applied: this ticket's primary golden should target the live apple/eat path first; any water/drink variant depends on 004 landing.

## Architecture Check

1. Follows existing golden test patterns: set up harness state with agent, facility, resource source, and recipes; run ticks; assert need satisfaction and commodity transfer. Primary proof should use the live apple/eat chain rather than assuming the still-unowned water contract.
2. No backward-compatibility shims. New test file, no modifications to existing tests.

## Verification Layers

1. Agent plans harvest when hungry and at apple resource source → decision trace: `AcquireCommodity(SelfConsume)` goal appears and Harvest action is planned
2. Harvest produces commodity in agent possession → authoritative world state: `agent_commodity_qty(agent, commodity) > 0` after harvest ticks
3. Agent consumes commodity and need decreases → authoritative world state: hunger decreases after consume ticks
4. Plan search within budget → decision trace: plan found within `max_node_expansions` (224) expansions

## What to Change

### 1. Create golden test for harvest-to-consume chain

Add a new test (either in `crates/worldwake-ai/tests/golden_production.rs` or a new `golden_harvest_consume.rs` file) that:

1. **Setup**: Create a scenario with:
   - One place with a Facility that has a `ResourceSource { commodity: Apple, available_quantity: Quantity(5), ... }` and appropriate `WorkstationMarker` (`OrchardRow`)
   - One agent at that place with:
     - `KnownRecipes` containing the "Harvest Apples" recipe ID
     - `HomeostaticNeeds` with high hunger (near threshold)
     - `DriveThresholds` configured to trigger hunger drive
     - `PerceptionProfile` (required to observe post-harvest entities)
     - Default `CognitiveProfile` (224 max expansions)
   - The "Harvest Apples" recipe registered in the `RecipeRegistry`

2. **Execute**: Run ticks until the agent satisfies hunger or a reasonable tick limit is reached.

3. **Assert**:
   - Agent's hunger decreased (need was satisfied)
   - `ResourceSource.available_quantity` decreased (harvest consumed from source)
   - Plan search completed within budget (no budget exhaustion)
   - Event log contains both harvest and eat events for the agent

### 2. (Optional) Add water/drink variant after S79RESSOUCON-004

If `S79RESSOUCON-004` lands the lawful water-source harvest contract, add a second test case with a water resource source and drink action.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify — add test) or `crates/worldwake-ai/tests/golden_harvest_consume.rs` (new)

## Out of Scope

- Testing harvest without recipe knowledge (that's a negative test — agents correctly fail, already covered by existing golden test constraints)
- Testing multi-agent harvest contention (covered by existing `golden_production.rs` tests)
- Testing exploration-driven harvest (deferred to S80)
- Testing craft → consume chains (only harvest → consume is in scope)

## Acceptance Criteria

### Tests That Must Pass

1. Golden test: agent with `KnownRecipes` and hunger at an Apple resource source plans and executes harvest → eat within 224 expansions
2. Golden test: agent's hunger level decreases after the harvest → eat chain completes
3. Golden test: `ResourceSource.available_quantity` decreases after harvest
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Agent plans from beliefs only — no direct world state access (P14)
2. Plan search completes within default `CognitiveProfile` budget of 224 expansions (P20)
3. Harvest → consume is a two-goal chain: `AcquireCommodity(SelfConsume)` then `ConsumeOwnedCommodity` — not a single-goal search
4. Any water/drink golden remains blocked on `S79RESSOUCON-004`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs` or `golden_harvest_consume.rs` — golden E2E test proving the full apple harvest-to-consume pipeline works end-to-end with live recipe knowledge and planner hypothetical state

### Commands

1. `cargo test -p worldwake-ai -- harvest_consume` (or the specific test name)
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
