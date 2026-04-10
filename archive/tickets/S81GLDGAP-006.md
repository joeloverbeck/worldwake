# S81GLDGAP-006: Golden test S81-C -- harvest-to-consume chain

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: None

## Problem

No golden test verifies the full harvest-to-consume action chain at resource source locations. S76-C (`golden_perception_forms_resource_source_beliefs`) tests belief formation about resource sources but not affordance generation or the harvest -> eat/drink execution chain. S79 tickets closed the runtime contract gaps, but there is no E2E test proving agents can plan and execute: perceive resource source -> harvest -> possess commodity -> consume -> need satisfied.

## Assumption Reassessment (2026-04-09)

1. S76-C test at `crates/worldwake-ai/tests/golden_perception_exposure.rs:442` tests belief formation, not affordance generation or consumption. Confirmed via grep.
2. S79 archived at `archive/specs/S79-resource-source-consumption-affordances.md`. Tickets S79RESSOUCON-003 and S79RESSOUCON-004 archived. The harvest-to-consume runtime path should now be functional.
3. `WorkstationTag::Well` and `OrchardRow` exist in `crates/worldwake-core/src/production.rs`. `ResourceSource` component at production.rs:75.
4. `KnownRecipes` component at `crates/worldwake-core/src/production.rs:42`. Recipe IDs for harvest actions must be discovered from existing test setups or recipe registry.
5. `golden_simulation_gaps.rs` exists. S81-C test will be added alongside S81-A and S81-B.
6. Live harness correction: `GoldenHarness::new()` still uses `build_recipes()` from `crates/worldwake-ai/tests/golden_harness/mod.rs:527-531`, which only registers `Harvest Apples`. Any scenario that needs `Harvest Water` must use `GoldenHarness::with_recipes(seed, build_multi_recipe_registry())` or equivalent.
7. Existing-coverage correction: the local apple `harvest -> materialize -> pick_up -> eat` branch is already proved in `crates/worldwake-ai/tests/golden_production.rs:3338-3478`. The remaining honest simulation-gap slice is the colocated resource-source harvest-to-consume chain with explicit water/drink coverage, while the apple branch can remain in-scenario as a symmetry/control proof rather than the novel gap by itself.
8. Live proof-surface correction: for a same-place resource-source scenario, the strongest early-layer AI proof is the opening decision trace showing `AcquireCommodity(SelfConsume)` selection with `Harvest` as the first planned op, not a vague "affordance set includes harvest" claim.
9. Scenario numbering correction: S81-A and S81-B landed as Scenarios 130 and 131 in `crates/worldwake-ai/tests/golden_simulation_gaps.rs`, so S81-C must use the next free repo-global id.
10. Golden-doc handoff correction: adding S81-C metadata changes the generated golden inventory/docs. Per `docs/golden-e2e-testing.md`, verification must include `python3 scripts/golden_inventory.py --write --check-docs`.
11. Scenario isolation: this test places agents directly at resource source locations to isolate the harvest-to-consume branch. Travel is excluded (agents are co-located with sources). The contract is: "given resource sources and recipe knowledge, agents can plan and execute harvest -> consume." Discovery/perception of sources is not under test (that is S76-C).

## Architecture Check

1. This golden test exercises the Production -> Needs system chain through state (P26). Agents harvest via production actions, then consume via needs actions. No new system coupling.
2. No backward-compatibility shims.

## Verification Layers

1. Opening planning trace selects the expected self-consume acquisition branch with `Harvest` as the first planned op -> decision trace
2. Agent executes harvest -> action trace (`ActionDomain::Production` committed)
3. Agent executes consume (`eat`/`drink`) -> action trace (`ActionDomain::Needs` committed)
4. Need level decreased after consumption -> authoritative world state (`HomeostaticNeeds` comparison)
5. Multi-layer golden E2E ticket: the contract spans production (harvest), inventory (commodity possession/materialization), and needs (consumption).

## What to Change

### 1. Add S81-C golden test to golden_simulation_gaps.rs

In `crates/worldwake-ai/tests/golden_simulation_gaps.rs`, add:

- Helper function `run_harvest_to_consume(seed: Seed)`:
  - Use `GoldenHarness::with_recipes(seed, build_multi_recipe_registry())` so both harvest recipes are live.
  - Create Agent A at a location with a Water resource source on a Well workstation. Give Agent A only the Harvest Water recipe knowledge.
  - Create Agent B at a location with an Apple resource source on an OrchardRow workstation. Give Agent B only the Harvest Apples recipe knowledge.
  - Set elevated hunger (for Agent B) and thirst (for Agent A) needs
  - Add `PerceptionProfile` and `CognitiveProfile` to both agents, and seed direct local beliefs so the scenario proves the harvest-to-consume chain rather than local discovery/perception lag
  - Run for 100 ticks
  - Assert: opening planning selects the expected self-consume acquisition branch for each agent, with `Harvest` as the first planned op
  - Assert: Agent A successfully harvests water and drinks within 100 ticks (`HomeostaticNeeds.thirst` decreased)
  - Assert: Agent B successfully harvests apples and eats within 100 ticks (`HomeostaticNeeds.hunger` decreased)

- Test function `golden_harvest_to_consume()` with `#[test]`
- Deterministic replay test `golden_harvest_to_consume_replays_deterministically()`

### 2. Follow existing golden test patterns

Use the same world-builder utilities and resource source setup patterns from S76-C's `run_perception_forms_resource_source_beliefs` as a template for resource source and workstation creation. The key difference is: this test asserts on action completion and need satisfaction, not belief formation.

## Files to Touch

- `crates/worldwake-ai/tests/golden_simulation_gaps.rs` (modify -- add S81-C test)

## Out of Scope

- Belief formation about resource sources (S76-C already covers)
- Travel to resource locations (S81-A covers multi-agent travel)
- Fixing affordance generation bugs (S79 already completed)
- Need-based mortality (S81GLDGAP-003/005)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_harvest_to_consume` -- colocated water/drink and apple/eat harvest-to-consume branches both complete within 100 ticks, with opening planning proving the harvest branch
2. `golden_harvest_to_consume_replays_deterministically` -- same seed produces same outcome
3. Generated golden docs updated cleanly for the new scenario metadata
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Harvest actions require recipe knowledge (P8 preconditions) -- agents without recipes cannot harvest
2. Consumption requires possession (P8) -- agents must hold the commodity before consuming
3. Need satisfaction is a concrete state change, not an abstract score (P3)
4. Deterministic replay under same seed

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_simulation_gaps.rs` -- 2 new test functions (golden + replay)
2. Generated docs under `docs/generated/` refreshed by the canonical golden inventory script

### Commands

1. `cargo test -p worldwake-ai --test golden_simulation_gaps golden_harvest_to_consume`
2. `cargo test -p worldwake-ai --test golden_simulation_gaps golden_harvest_to_consume_replays_deterministically`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-04-10.

- Added Scenario 132 in `crates/worldwake-ai/tests/golden_simulation_gaps.rs` covering colocated resource-source harvest-to-consume execution for both the water/drink and apple/eat branches.
- The scenario now proves the strongest honest AI-layer opening boundary for this slice: each agent selects a self-consume `AcquireCommodity` harvest plan with `Harvest` as the first planned op, then commits the matching harvest and consume actions, and the relevant need decreases.
- Regenerated the golden inventory/docs so Scenario 132 is reflected in the generated coverage artifacts under `docs/generated/`.

## Deviations

- Reassessment showed `GoldenHarness::new()` does not register `Harvest Water`, so the scenario uses `GoldenHarness::with_recipes(seed, build_multi_recipe_registry())` rather than the default harness constructor.
- Reassessment also showed the local apple harvest-to-eat branch is already covered in `golden_production.rs`. This ticket therefore owns the remaining simulation-gap proof surface: explicit local water/drink coverage plus a colocated dual-branch harvest-to-consume scenario, rather than claiming the apple branch was entirely unproved.
- The early proof surface was tightened from a vague "affordance includes harvest" assertion to the live decision-trace contract: opening planning selects the expected self-consume harvest branch with `Harvest` as the first planned op.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_simulation_gaps golden_harvest_to_consume`
- Passed `cargo test -p worldwake-ai --test golden_simulation_gaps golden_harvest_to_consume_replays_deterministically`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai`
