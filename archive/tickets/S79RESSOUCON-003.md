# S79RESSOUCON-003: Golden test — harvest-to-consume chain

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S79RESSOUCON-001, S79RESSOUCON-002, S79RESSOUCON-004

## Problem

The focused fixes in tickets 001 and 002 need an owning golden proof at the correct causal boundary. Existing `golden_production.rs` scenarios already proved the apple harvest/materialize/pick-up/eat chain through authoritative state, but they did not yet prove the planner-side branch selection and search-budget contract that S79 depends on: `AcquireCommodity(SelfConsume)` must be selected from fresh search, the winning plan must include `Harvest`, and the default `CognitiveProfile.max_node_expansions` budget must be sufficient for the live apple path.

## Assumption Reassessment (2026-04-09)

1. Existing golden test infrastructure in `crates/worldwake-ai/tests/`: `golden_production.rs` already contains live apple-chain proofs, including `golden_materialization_barrier_chain()` for the local `harvest -> materialize -> pick-up -> eat` path and `golden_faction_ownership_producer_owner_delegation()` for a faction-owned `harvest -> pickup -> eat` branch. The stale gap is narrower than originally stated: the owning apple golden needed planner-side decision-trace and default-budget assertions, not a second duplicate apple scenario.
2. `GoldenHarness` (in `crates/worldwake-ai/tests/golden_harness/mod.rs`) provides `with_recipes()`, `agent_commodity_qty()`, and recipe lookup by name. Agents need `PerceptionProfile` to observe newly created entities from production.
3. Shared boundary: golden E2E harness → full tick simulation → candidate generation + planner + action execution. This test exercises the complete pipeline from scenario setup to need satisfaction.
4. This is still a golden E2E ticket, but the live gap belongs in the existing owner scenario rather than a new file. The `GoalKind` under test is `AcquireCommodity { commodity, purpose: SelfConsume }` leading to `ConsumeOwnedCommodity`. The planner uses `ACQUIRE_OPS` (including `PlannerOpKind::Harvest`) and `CONSUME_OPS` (including `PlannerOpKind::Consume`).
5. Budget: default `CognitiveProfile.max_node_expansions` is 224. The test must verify the plan completes within this budget.
6. Ticket 001 completed the scenario/bootstrap path only for the currently canonical production recipes (`Harvest Apples`, `Harvest Grain`, `Bake Bread`). The lawful water harvest contract remains uncovered and is now owned by `S79RESSOUCON-004`. Correction applied: this ticket's primary golden should target the live apple/eat path first; any water/drink variant depends on 004 landing.

## Architecture Check

1. Follows existing golden test patterns: set up harness state with agent, facility, resource source, and recipes; run ticks; assert need satisfaction and commodity transfer. Primary proof should use the live apple/eat chain rather than assuming the still-unowned water contract.
2. No backward-compatibility shims. Modify the existing owner scenario in `golden_production.rs`; do not create a duplicate harvest/eat file.

## Verification Layers

1. Agent plans harvest when hungry and at apple resource source → decision trace: in the earliest fresh-search planning window after local observation, `AcquireCommodity(SelfConsume)` is selected and the selected plan includes `Harvest`
2. Harvest consumes from the orchard source → authoritative world state: `ResourceSource.available_quantity` decreases after harvest ticks
3. Agent consumes the harvested commodity and hunger decreases → authoritative world state: hunger decreases after the barrier chain completes
4. Plan search within budget → selected-plan provenance: `expansions_used <= max_node_expansions` (224 default)

## What to Change

### 1. Create golden test for harvest-to-consume chain

Strengthen the existing owner scenario in `crates/worldwake-ai/tests/golden_production.rs` so it proves the missing planner-side contract on top of the already-present authoritative apple-chain proof:

1. **Setup**: Reuse the existing colocated orchard scenario in `golden_materialization_barrier_chain()`.
2. **Assert in the earliest fresh-search harvest planning trace**:
   - `AcquireCommodity(SelfConsume){Apple}` is the selected goal
   - `selected_plan_source == SearchSelection`
   - the winning plan includes `PlannerOpKind::Harvest`
   - `selected_plan.search_provenance.expansions_used <= 224`
3. **Execute**: Run ticks until the materialization barrier chain completes.
4. **Assert after execution**:
   - `ResourceSource.available_quantity` decreased
   - hunger decreased after the harvest → materialize → pick-up → eat chain

### 2. (Optional) Add water/drink variant after S79RESSOUCON-004

If `S79RESSOUCON-004` lands the lawful water-source harvest contract, add a second test case with a water resource source and drink action.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify existing owner scenario)

## Out of Scope

- Testing harvest without recipe knowledge (that's a negative test — agents correctly fail, already covered by existing golden test constraints)
- Testing multi-agent harvest contention (covered by existing `golden_production.rs` tests)
- Testing exploration-driven harvest (deferred to S80)
- Testing craft → consume chains (only harvest → consume is in scope)

## Acceptance Criteria

### Tests That Must Pass

1. Golden test: existing apple materialization barrier scenario reaches a fresh-search planning trace that selects `AcquireCommodity(SelfConsume){Apple}` and whose winning plan includes `Harvest`
2. Golden test: selected-plan search provenance stays within the default 224-node expansion budget
3. Golden test: agent's hunger level decreases after the harvest → eat chain completes
4. Golden test: `ResourceSource.available_quantity` decreases after harvest
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Agent plans from beliefs only — no direct world state access (P14)
2. Plan search completes within default `CognitiveProfile` budget of 224 expansions (P20)
3. Harvest → consume is a two-goal chain: `AcquireCommodity(SelfConsume)` then `ConsumeOwnedCommodity` — not a single-goal search
4. Any water/drink golden remains blocked on `S79RESSOUCON-004`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs` — strengthen `golden_materialization_barrier_chain()` with decision-trace and search-budget assertions for the live apple harvest-to-consume pipeline

### Commands

1. `cargo test -p worldwake-ai golden_materialization_barrier_chain`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completion date: 2026-04-09

Updated the existing owner golden in `golden_production.rs` instead of adding a duplicate apple scenario. `golden_materialization_barrier_chain()` now proves the missing planner-side contract in the earliest fresh-search harvest trace (`AcquireCommodity(SelfConsume){Apple}` selected from fresh search, winning plan includes `Harvest`, and `expansions_used <= 224`) while preserving the authoritative end-to-end proof that orchard quantity decreases and hunger falls after the full barrier chain completes.

Deviation from original plan: reassessment showed the apple harvest-to-consume chain was already covered authoritatively in existing `golden_production.rs` scenarios, so the implemented slice strengthened the existing owner golden rather than creating a new standalone harvest/eat test file.

## Verification Result

- `cargo test -p worldwake-ai golden_materialization_barrier_chain`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
