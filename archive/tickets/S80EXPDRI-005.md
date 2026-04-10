# S80EXPDRI-005: Golden E2E tests

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes
**Deps**: S80EXPDRI-001, S80EXPDRI-002, S80EXPDRI-003, S80EXPDRI-004

## Problem

The exploration drive currently has focused unit coverage but no dedicated golden coverage for the ignorance-first fallback path that landed in `S80EXPDRI-004`. Golden tests are needed to prove the full causal chain under the live contract: unmet self-care need + no known satisfaction path + known frontier place → `ExploreLocation` goal → travel → perception → belief update → downstream satisfaction path unlocked. Without these tests, regressions in the need→exploration→discovery pipeline would go undetected.

## Assumption Reassessment (2026-04-10)

1. Golden E2E tests live in `crates/worldwake-ai/tests/golden_*.rs`. Existing nearest coverage in `golden_simulation_gaps.rs` proves remote scarcity behavior when remote resources are already believed, but it does not cover ignorance-triggered exploration or post-arrival belief acquisition. A dedicated `golden_exploration.rs` file is justified because no existing suite owns the exploration-specific fallback contract.
2. Golden tests that rely on post-arrival observation require an explicit `PerceptionProfile`; otherwise the agent can legally fail to observe destination contents. `docs/golden-e2e-testing.md` and `AGENTS.md` are the canonical references for this harness rule.
3. Shared abstraction boundary: these goldens exercise the full agent tick pipeline (candidate generation → ranking → planning → execution → perception → belief update). They validate the complete causal chain, not individual layers.
4. The live exploration contract is narrower than the original spec draft. Exploration is currently emitted only for self-care needs, and only when no non-self-care candidate families already exist. Golden expectations must match that narrowed contract rather than the earlier “competes with other goals” framing.
5. Exploration target choice is now deterministic, not RNG-based. The live ordering is frontier novelty first, then proximity, then oldest surviving place belief, then stable entity-id order. Golden scenarios should assert deterministic selection where relevant.
6. Scenario isolation remains essential: ignorance-path tests must withhold food or water source beliefs while still providing a known frontier place belief, and the “known path exists” variant must seed a lawful believed source so the test proves suppression rather than topology failure.
7. `python3 scripts/golden_inventory.py --write --check-docs` only includes scenarios that declare repo-global `// Scenario ...` metadata blocks in generator-friendly format. Because this ticket adds a new golden file, refreshing `docs/generated/` is part of the owned verification surface rather than optional follow-up cleanup.

## Architecture Check

1. Golden tests should follow the established pattern: create scenario → set up explicit beliefs/profiles → advance ticks → assert on decision traces, action traces, and belief state. No new test infrastructure is needed.
2. `ExplorationProfile` is universal and already present on agents by default, but scenarios may still override `max_consecutive_explorations` or thresholds when the cap itself is the contract under test.
3. Live reassessment uncovered a production contradiction: `ExploreLocation` was still registered under `SOCIAL_POLICY`, which suppressed exploration under the same self-care stress that emits it. The ticket therefore requires a minimal production fix in `goal_dispatch_decl.rs` in addition to the new goldens.
4. No backward-compatibility shims.

## Verification Layers

1. Exploration triggers on need + ignorance → decision trace: `ExploreLocation` candidate present on the opening tick, and the planner can convert it into a travel plan under the narrowed self-care fallback contract.
2. No exploration when a lawful satisfaction path already exists → decision trace: `ExploreLocation` candidate absent and a concrete self-care path present instead.
3. Consecutive cap respected → decision trace: `ExploreLocation` candidate absent once the adopted-goal counter reaches the profile cap.
4. Arrival yields new beliefs → action trace plus belief state: travel commits to the frontier place, then the agent’s `AgentBeliefStore` gains beliefs about destination resource entities and subsequent planning shifts to concrete acquisition/consumption.
5. Where target selection is asserted, use decision traces and the deterministic frontier-order contract rather than randomness assumptions.

## What to Change

### 1. Scenario setup helpers

Create focused exploration scenario helpers:
- Agent with configurable `ExplorationProfile` and explicit `PerceptionProfile`
- Place graph with at least a start place, a known intermediate or adjacent frontier belief as needed, and a destination resource place
- Resource source at the exploration target for the belief-unlock scenario
- Agent starts with elevated hunger or thirst, local beliefs only, and no believed satisfaction path

### 2. Test: Exploration triggers on need + ignorance

**Setup**: Agent at Place A with hunger above the exploration activation threshold. The agent believes a frontier place exists but has no believed food sources and no lawful non-self-care candidate families.

**Assert**: On the opening planning tick, the decision trace shows `ExploreLocation { target_place: B, motivating_need: Hunger }` as a generated and selected goal.

### 3. Test: No exploration when satisfaction path exists

**Setup**: Same as test 1, but the agent has a belief that a lawful food source already exists at the current or otherwise known reachable place.

**Assert**: The decision trace shows no `ExploreLocation` candidate, and a concrete hunger-relief candidate such as `AcquireCommodity` or `ConsumeOwnedCommodity` is generated instead.

### 4. Test: Consecutive cap respected

**Setup**: Agent with `max_consecutive_explorations: 1` or `2`, with the profile counter pre-seeded to the cap or advanced through adopted exploration goals in-scenario.

**Assert**: Once the counter reaches the cap, the next planning trace generates no `ExploreLocation` candidate. The scenario should avoid asserting an artificial fallback if the lawful result is simply “no exploration emitted”.

### 5. Test: Arrival yields new beliefs

**Setup**: Agent begins hungry at Place A with a frontier-place belief for Place B but no believed resource source there. Place B authoritatively contains a food-producing resource source, and the agent has a `PerceptionProfile` capable of observing it on arrival.

**Assert**: After travel commits and post-arrival perception runs, the agent’s `AgentBeliefStore` contains beliefs about the destination source or workstation, and a subsequent planning trace shifts from exploration to concrete hunger relief.

## Files to Touch

- `crates/worldwake-ai/tests/golden_exploration.rs` (new — focused exploration goldens)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (register `ExploreLocation` under the self-care family policy)
- `docs/generated/golden-e2e-inventory.md` (regenerated golden inventory)
- `docs/generated/golden-scenario-index.md` (regenerated scenario index)
- `docs/generated/golden-coverage-matrix.md` (regenerated scenario metadata coverage)
- `docs/generated/golden-scenario-details/exploration.md` (new per-file scenario detail output)
- `docs/generated/golden-scenario-details/{production,simulation-gaps,social,supply-chain}.md` (line-reference churn from regenerated inventory output)

## Out of Scope

- Systematic cartography or map-building golden tests
- Exploration with multiple agents (social information propagation)
- Performance benchmarking of exploration target selection
- Soak-test scenarios with exploration

## Acceptance Criteria

### Tests That Must Pass

1. `golden_exploration::golden_exploration_triggers_on_need_and_ignorance`
2. `golden_exploration::golden_exploration_is_suppressed_when_known_satisfaction_path_exists`
3. `golden_exploration::golden_exploration_consecutive_cap_is_respected`
4. `golden_exploration::golden_exploration_arrival_unlocks_beliefs_and_concrete_relief`
5. `worldwake_ai::goal_dispatch_decl::tests::explore_location_uses_self_care_policy`
6. Existing suite: `cargo test -p worldwake-ai`
7. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. All planning assertions use belief-mediated reads only; authoritative state is used only to validate world-side setup and observed consequences after lawful perception (P14).
2. All golden agents that must observe destination contents have an explicit `PerceptionProfile`.
3. Each test isolates one causal branch so failures point to exploration behavior rather than unrelated scarcity, recipe, or topology issues (P31).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_exploration.rs` — 4 scenarios validating the narrowed exploration fallback causal chain
2. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — register `ExploreLocation` under the self-care family policy so the emitted fallback is not suppressed under self-care stress
3. `docs/generated/` golden inventory outputs — refreshed after adding scenario metadata for the new exploration suite

### Commands

1. `cargo test -p worldwake-ai --test golden_exploration`
2. `cargo test -p worldwake-ai`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo build --workspace`

## Outcome

Completed on 2026-04-10.

- Added `crates/worldwake-ai/tests/golden_exploration.rs` with four exploration goldens covering ignorance-triggered exploration, suppression when a lawful satisfaction path already exists, the consecutive-exploration cap, and post-arrival belief unlock shifting planning to concrete relief.
- Fixed the live policy contradiction by registering `ExploreLocation` under `SELF_CARE_POLICY` and added `explore_location_uses_self_care_policy` to keep that dispatch contract under test.
- Added repo-global scenario metadata for the new exploration suite and refreshed `docs/generated/` so the golden inventory, scenario index, coverage matrix, and per-file scenario details now include the exploration scenarios.
- Regenerating the scenario docs also updated nearby generated line references in existing detail/index files (`production`, `simulation-gaps`, `social`, and `supply-chain`) because the inventory is source-position driven.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_exploration`
- Passed `cargo test -p worldwake-ai`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo build --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
