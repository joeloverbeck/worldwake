# S81GLDGAP-006: Golden test S81-C -- harvest-to-consume chain

**Status**: PENDING
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
8. Scenario isolation: this test places agents directly at resource source locations to isolate the harvest-to-consume branch. Travel is excluded (agents are co-located with sources). The contract is: "given resource sources and recipe knowledge, agents can plan and execute harvest -> consume." Discovery/perception of sources is not under test (that is S76-C).

## Architecture Check

1. This golden test exercises the Production -> Needs system chain through state (P26). Agents harvest via production actions, then consume via needs actions. No new system coupling.
2. No backward-compatibility shims.

## Verification Layers

1. Affordance set includes harvest action -> decision trace (affordance_query output)
2. Agent executes harvest -> action trace (ActionDomain::Production committed)
3. Agent executes consume (eat/drink) -> action trace (ActionDomain::Needs committed)
4. Need level decreased after consumption -> authoritative world state (HomeostaticNeeds comparison)
5. Multi-layer golden E2E ticket: the contract spans production (harvest), inventory (commodity possession), and needs (consumption).

## What to Change

### 1. Add S81-C golden test to golden_simulation_gaps.rs

In `crates/worldwake-ai/tests/golden_simulation_gaps.rs`, add:

- Helper function `run_harvest_to_consume(seed: Seed)`:
  - Create Agent A at a location with a Water resource source on a Well workstation. Give Agent A the Harvest Water recipe knowledge.
  - Create Agent B at a location with an Apple resource source on an OrchardRow workstation. Give Agent B the Harvest Apples recipe knowledge.
  - Set elevated hunger (for Agent B) and thirst (for Agent A) needs
  - Add `PerceptionProfile` to both agents (required for observing post-production output)
  - Run for 100 ticks
  - Assert: Agent A successfully harvests water and drinks within 100 ticks (HomeostaticNeeds.thirst decreased)
  - Assert: Agent B successfully harvests apples and eats within 100 ticks (HomeostaticNeeds.hunger decreased)

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

1. `golden_harvest_to_consume` -- both agents harvest and consume within 100 ticks, needs decrease
2. `golden_harvest_to_consume_replays_deterministically` -- same seed produces same outcome
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Harvest actions require recipe knowledge (P8 preconditions) -- agents without recipes cannot harvest
2. Consumption requires possession (P8) -- agents must hold the commodity before consuming
3. Need satisfaction is a concrete state change, not an abstract score (P3)
4. Deterministic replay under same seed

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_simulation_gaps.rs` -- 2 new test functions (golden + replay)

### Commands

1. `cargo test -p worldwake-ai -- golden_harvest_to_consume`
2. `cargo test -p worldwake-ai`
