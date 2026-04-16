# S105OBSSALFIL-003: Unit and golden tests for observation salience filtering

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: archive/tickets/S105OBSSALFIL-002.md (completed)

## Problem

The observation priority and budget pipeline added in S105OBSSALFIL-002 has no dedicated test coverage. The existing test suite verifies behavioral equivalence at the default budget (24), but does not exercise priority ordering, budget truncation, or the interaction between need-based boosting and Waste exclusion. This ticket adds focused unit tests and a golden E2E scenario that prove the pipeline works correctly at non-default budgets.

## Assumption Reassessment (2026-04-16)

1. After S105OBSSALFIL-002, `collect_direct_local_observation_batch` accepts `needs: HomeostaticNeeds` and `profile: &PerceptionProfile` parameters and implements priority sorting + budget truncation. `compute_observation_priority` is a private function in `perception.rs`.
2. The `#[cfg(test)]` module in `perception.rs` starts at line 1164. Existing tests import `PerceptionProfile`, `HomeostaticNeeds`, `EntityKind`, `CommodityKind`, and world-building utilities — all needed for the new unit test.
3. `WorldTxn` in `worldwake-core` supports `create_entity` with `EntityKind::ItemLot`, `set_component_item_lot`, and `set_component_homeostatic_needs` — all needed to construct the test world.
4. Golden E2E tests live in `crates/worldwake-ai/tests/golden_*.rs`. The `golden_harness` module provides world-building helpers. `WorkstationTag::Well` and `WorkstationTag::OrchardRow` exist as enum variants (verified during reassessment).
5. Existing golden test scenarios use at most ~20 entities per place (verified during reassessment). Default `observation_budget = 24` will not trigger truncation in any existing test, confirming regression safety.
6. `DirectLocalObservationBatch` contains `observed_snapshots: BTreeMap<EntityId, BelievedEntityState>` — the observation results can be inspected by checking which EntityIds appear in the batch.

## Architecture Check

1. Unit tests exercise the pipeline through internal function calls with controlled world state — no mocking, no faking. The golden E2E test runs through the full perception system with a budget below entity count, proving end-to-end correctness.
2. No backward-compatibility shims. Tests verify the new behavior directly.

## Verification Layers

1. Priority ordering (Agent > Facility > Waste) → focused unit test with known entity composition at budget < total entities
2. Budget truncation at `observation_budget` → focused unit test asserting batch size ≤ budget
3. Deterministic tie-breaking by EntityId → focused unit test with same-priority entities
4. Need-based boost for non-Waste ItemLots → focused unit test with high need pressure agent
5. E2E perception correctness under budget pressure → golden test with 2 agents, facilities, and 40 Waste items at budget 12
6. Regression safety → verification that default budget (24) does not trigger in any existing golden scenario

## What to Change

### 1. Unit test: priority ordering and budget truncation

In `crates/worldwake-systems/src/perception.rs` test module, add a test that:
- Creates a world with 1 observer agent, 1 other Agent, 2 Facilities, and 30 Waste ItemLots at the same place
- Sets `observation_budget = 10` on the observer's `PerceptionProfile`
- Calls `collect_direct_local_observation_batch` with fidelity 1000 (guaranteed pass)
- Asserts:
  - Batch contains exactly 10 observed entities (budget)
  - The other Agent entity is in the batch (priority 900)
  - Both Facilities are in the batch (priority 700)
  - At most 7 Waste entities are in the batch (budget remainder)
  - Waste entities in the batch are ordered by EntityId (deterministic tie-breaking)

### 2. Unit test: need-based boost for non-Waste ItemLots

Add a test that:
- Creates a world with 1 observer, 5 non-Waste ItemLots (e.g., Apple), and 10 Waste ItemLots at the same place
- Sets `observation_budget = 8`, `need_salience_urgency_threshold = 400`, `need_salience_boost = 500`
- Sets observer's `HomeostaticNeeds` with hunger at 800 (above threshold)
- Calls `collect_direct_local_observation_batch`
- Asserts:
  - All 5 Apple ItemLots are in the batch (boosted priority: 300 + boost > Waste's 100)
  - At most 3 Waste entities fill the remaining budget
  - No Apple ItemLot is excluded before all Waste if budget permits

### 3. Golden E2E test: perception under budget pressure

In a new test in `crates/worldwake-ai/tests/golden_perception_exposure.rs` (existing file for perception-focused golden tests), add a test that:
- Creates a scenario: 2 agents at 1 place with a Well facility, an OrchardRow facility, and 40 pre-placed Waste ItemLots
- Sets `observation_budget = 12` for both agents
- Runs for 20 ticks
- Asserts:
  - Both agents observe each other (Agent at priority 900 always in budget)
  - Both agents observe the Well and OrchardRow facilities
  - Total unique Waste entities in each agent's belief store is bounded (significantly less than 40)
  - Agents still satisfy basic survival needs (perception budget doesn't starve them of resource awareness)

### 4. Regression check

Verify that default `observation_budget = 24` does not alter any existing golden test output:
- Run `cargo test -p worldwake-ai` — all existing tests must pass with identical results
- Spot-check entity counts per place in existing golden scenarios to confirm all are below 24

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify — add unit tests to `#[cfg(test)]` module)
- `crates/worldwake-ai/tests/golden_perception_exposure.rs` (modify — add golden E2E test)

## Out of Scope

- Observer validation on `survival-baseline.ron` (operational check, not a committed test)
- Testing goal-specific observation filtering (non-goal per spec)
- Testing dynamic budget adjustment (non-goal per spec)
- Testing observation priority for evidence entries or scene elements
- Performance benchmarking of the sort-and-truncate overhead

## Acceptance Criteria

### Tests That Must Pass

1. New unit test: priority ordering respects EntityKind hierarchy and budget truncation
2. New unit test: need-based boost activates for non-Waste ItemLots above urgency threshold
3. New golden test: agents observe high-priority entities under budget pressure, Waste belief count is bounded
4. All existing golden tests pass unchanged (regression)
5. `cargo clippy --workspace --all-targets -- -D warnings` passes

### Invariants

1. At `observation_budget = N`, no agent observes more than N entities per tick (excluding the place itself)
2. Priority ordering: Agent > Place > Facility > UniqueItem > Office > Container > Faction > Record = SocialArtifact > ItemLot > Waste ItemLot
3. Deterministic: same seed + same entity composition = same observation batch
4. Need boost applies only to non-Waste ItemLots when `max_need >= need_salience_urgency_threshold`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs::tests::test_observation_budget_truncation_and_priority` — proves priority ordering and budget cap
2. `crates/worldwake-systems/src/perception.rs::tests::test_observation_need_boost_non_waste` — proves need-based boost for non-Waste ItemLots
3. `crates/worldwake-ai/tests/golden_perception_exposure.rs::golden_observation_budget_pressure` — proves E2E perception correctness under budget constraint

### Commands

1. `cargo test -p worldwake-systems -- test_observation_budget`
2. `cargo test -p worldwake-systems -- test_observation_need_boost`
3. `cargo test -p worldwake-ai -- golden_observation_budget`
4. `cargo test -p worldwake-ai` (full golden suite regression)
5. `cargo clippy --workspace --all-targets -- -D warnings`
