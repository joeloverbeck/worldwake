# S80EXPDRI-005: Golden E2E tests

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S80EXPDRI-001, S80EXPDRI-002, S80EXPDRI-003, S80EXPDRI-004

## Problem

The exploration drive system has no end-to-end validation. Golden tests are needed to prove the full causal chain: unmet need + geographic ignorance → ExploreLocation goal → travel → perception → belief update → downstream satisfaction path unlocked. Without these tests, regressions in the need→exploration→discovery pipeline would go undetected.

## Assumption Reassessment (2026-04-10)

1. Golden E2E tests live in `crates/worldwake-ai/tests/golden_*.rs`. Existing patterns: `golden_simulation_gaps.rs`, `golden_perception_exposure.rs`, `golden_reasoning_diversity.rs`. These use full simulation setup with scenarios, tick advancement, and assertion on decision traces / world state.
2. Golden tests require `PerceptionProfile` on agents that need to observe post-arrival entities. Without it, agents silently fail to perceive new location contents. This is a documented gotcha in CLAUDE.md.
3. Shared abstraction boundary: golden tests exercise the full agent tick pipeline (candidate generation → ranking → planning → execution → perception → belief update). They validate the complete causal chain, not individual layers.
12. Scenario isolation: each test isolates one branch by controlling initial beliefs. Test 1 (trigger on ignorance) removes food-source beliefs. Test 2 (no explore with known source) adds food-source beliefs. Test 3 (consecutive cap) runs multiple ticks. Test 4 (arrival beliefs) checks belief state after travel completes.

## Architecture Check

1. Golden tests follow the established pattern: create scenario → set up initial state → advance ticks → assert on traces/state. No new test infrastructure needed. ExplorationProfile is a universal profile, so all test agents get one automatically (with defaults or custom values).
2. No backward-compatibility shims. New test files only.

## Verification Layers

1. Exploration triggers on need + ignorance → decision trace: ExploreLocation candidate present in generate_candidates output
2. No exploration when satisfaction path exists → decision trace: ExploreLocation candidate absent
3. Consecutive cap respected → decision trace: ExploreLocation candidate absent after N consecutive explorations
4. Arrival yields new beliefs → authoritative world state: agent's AgentBeliefStore contains new place entity beliefs after travel completes
5. Multi-layer ticket: decision traces prove candidate generation, action traces prove travel execution, authoritative state proves belief acquisition.

## What to Change

### 1. Scenario setup helpers

Create test utilities for exploration scenarios:
- Agent with configurable `ExplorationProfile` (custom curiosity_weight, thresholds)
- Place graph with at least 2 places (start place + exploration target)
- Resource source at target place (for arrival-yields-beliefs test)
- Agent starts with high hunger but no belief about food sources

### 2. Test: Exploration triggers on need + ignorance

**Setup**: Agent at Place A with hunger above threshold. No believed food sources anywhere. Adjacent Place B exists in topology.

**Assert**: After candidate generation tick, decision trace shows `ExploreLocation { target_place: B, motivating_need: Hunger }` as a generated candidate.

### 3. Test: No exploration when satisfaction path exists

**Setup**: Same as test 1, but agent has a belief that Place A (current location) has a food-producing resource source.

**Assert**: After candidate generation tick, decision trace shows NO ExploreLocation candidate. AcquireCommodity or ConsumeOwnedCommodity candidate present instead.

### 4. Test: Consecutive cap respected

**Setup**: Agent with `max_consecutive_explorations: 2`. Agent starts exploring (tick 1 → ExploreLocation selected, tick 2 → ExploreLocation selected). On tick 3, agent should not generate another ExploreLocation.

**Assert**: After 2 consecutive ExploreLocation selections, the 3rd candidate generation produces no ExploreLocation candidate. Agent falls back to other goals.

### 5. Test: Arrival yields new beliefs

**Setup**: Agent at Place A, ExploreLocation goal targeting Place B. Place B has a food-producing resource source (e.g., orchard). Agent has PerceptionProfile.

**Assert**: After travel completes and perception fires, agent's `AgentBeliefStore` contains beliefs about the resource source at Place B. Subsequent candidate generation produces AcquireCommodity for food instead of ExploreLocation.

## Files to Touch

- `crates/worldwake-ai/tests/golden_exploration.rs` (new — all 4 golden test scenarios)

## Out of Scope

- Systematic cartography or map-building golden tests
- Exploration with multiple agents (social information propagation)
- Performance benchmarking of exploration target selection
- Soak-test scenarios with exploration

## Acceptance Criteria

### Tests That Must Pass

1. `golden_exploration::exploration_triggers_on_need_and_ignorance`
2. `golden_exploration::no_exploration_when_satisfaction_path_exists`
3. `golden_exploration::consecutive_exploration_cap_respected`
4. `golden_exploration::arrival_yields_new_beliefs_and_unlocks_satisfaction`
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. All golden tests use belief-mediated reads only — no test asserts on authoritative state that the agent couldn't have perceived (P14)
2. All golden agents have PerceptionProfile to avoid silent perception failures
3. Each test isolates one causal branch — setup controls ensure only the intended path is exercised (P31)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_exploration.rs` — 4 scenarios validating the full exploration drive causal chain

### Commands

1. `cargo test -p worldwake-ai golden_exploration`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo build --workspace`
