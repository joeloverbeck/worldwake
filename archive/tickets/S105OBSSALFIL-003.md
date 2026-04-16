# S105OBSSALFIL-003: Unit and golden tests for observation salience filtering

**Status**: COMPLETED
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
5. `DirectLocalObservationBatch.observed_snapshots` includes the observed place entity in addition to the budgeted non-place entity set whenever the place-level fidelity check passes. Focused budget assertions therefore must count `observed_snapshots` entries excluding `batch.place`, not use the raw map length.
6. `observed_snapshots` is a `BTreeMap<EntityId, BelievedEntityState>`, so iteration order is `EntityId`-sorted, not priority-sorted. Deterministic tie-breaking for same-priority entities must be proved by which entity IDs survive truncation, not by the order of map iteration.
7. `crates/worldwake-ai/tests/golden_perception_exposure.rs` already owns same-place perception goldens and already has reusable helpers for placing workstation resource sources, ground lots, stable-metabolism agents, and per-agent perception-profile overrides. This ticket can stay inside the existing file and does not need a new golden file.
8. The existing `golden_perception_forms_resource_source_beliefs` scenario already proves single-agent resource-source retention under local clutter. The remaining honest golden delta for this ticket is reduced-budget multi-agent/facility visibility plus bounded waste retention, not a separate survival-metabolism proof.

## Architecture Check

1. Unit tests exercise the pipeline through internal function calls with controlled world state — no mocking, no faking. The golden E2E test runs through the full perception system with a budget below entity count, proving end-to-end correctness.
2. No backward-compatibility shims. Tests verify the new behavior directly.

## Verification Layers

1. Priority ordering (Agent > Facility > Waste) → focused unit test with known entity composition at budget < total entities
2. Budget truncation at `observation_budget` → focused unit test asserting non-place observed entity count ≤ budget while place observation remains separate
3. Deterministic tie-breaking by EntityId → focused unit test proving the selected same-priority waste subset is the lowest `EntityId`s that fit under the budget
4. Need-based boost for non-Waste ItemLots → focused unit test with high need pressure agent
5. E2E perception correctness under budget pressure → golden test with 2 agents, facilities, and 40 Waste items at budget 12
6. Regression safety → existing `cargo test -p worldwake-ai` suite stays green at the default budget after the new focused golden lands

## What to Change

### 1. Unit test: priority ordering and budget truncation

In `crates/worldwake-systems/src/perception.rs` test module, add a test that:
- Creates a world with 1 observer agent, 1 other Agent, 2 Facilities, and 30 Waste ItemLots at the same place
- Sets `observation_budget = 10` on the observer's `PerceptionProfile`
- Calls `collect_direct_local_observation_batch` with fidelity 1000 (guaranteed pass)
- Asserts:
  - The batch contains exactly 10 observed non-place entities (budget) and also includes the place snapshot
  - The other Agent entity is in the batch (priority 900)
  - Both Facilities are in the batch (priority 700)
  - At most 7 Waste entities are in the batch (budget remainder)
  - The retained Waste entity set is the lowest-`EntityId` Waste subset that fits the remaining budget (deterministic tie-breaking)

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
  - Total retained Waste entities in each agent's belief store stays bounded well below 40 because the same lowest-priority subset is repeatedly selected under deterministic truncation

### 4. Regression check

Verify that the existing golden suite still passes with the default `observation_budget = 24` after the new tests land:
- Run `cargo test -p worldwake-ai`

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
3. New golden test: agents observe high-priority entities under budget pressure, and retained Waste belief count is bounded by deterministic truncation
4. All existing golden tests pass unchanged (regression)
5. `cargo clippy --workspace --all-targets -- -D warnings` passes

### Invariants

1. At `observation_budget = N`, no agent observes more than N entities per tick (excluding the place itself)
2. Priority ordering: Agent > Place > Facility > UniqueItem > Office > Container > Faction > Record = SocialArtifact > ItemLot > Waste ItemLot
3. Deterministic: same seed + same entity composition = same observation batch
4. Need boost applies only to non-Waste ItemLots when `max_need >= need_salience_urgency_threshold`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs::tests::passive_local_observation_applies_budget_priority_to_non_place_entities` — proves priority ordering, non-place budget cap, and same-priority tie-breaking
2. `crates/worldwake-systems/src/perception.rs::tests::passive_local_observation_boosts_non_waste_item_lots_when_needs_are_urgent` — proves need-based boost for non-Waste ItemLots
3. `crates/worldwake-ai/tests/golden_perception_exposure.rs::golden_observation_budget_prioritizes_agents_and_facilities_over_waste` — proves E2E perception correctness under budget constraint

### Commands

1. `cargo test -p worldwake-systems --lib perception::tests::passive_local_observation_applies_budget_priority_to_non_place_entities -- --exact`
2. `cargo test -p worldwake-systems --lib perception::tests::passive_local_observation_boosts_non_waste_item_lots_when_needs_are_urgent -- --exact`
3. `cargo test -p worldwake-ai --test golden_perception_exposure golden_observation_budget_prioritizes_agents_and_facilities_over_waste -- --exact`
4. `cargo test -p worldwake-systems`
5. `cargo test -p worldwake-ai`
6. `python3 scripts/golden_inventory.py --write --check-docs`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-16.

Added two focused unit tests in `crates/worldwake-systems/src/perception.rs` that exercise `collect_direct_local_observation_batch` directly under reduced budgets. One proves deterministic priority selection across Agent, Facility, and Waste entities while counting the budgeted non-place subset separately from the place snapshot. The second proves that urgent `HomeostaticNeeds` plus `need_salience_boost` lift non-Waste `ItemLot` observations ahead of Waste under the same budget pressure.

Added `golden_observation_budget_prioritizes_agents_and_facilities_over_waste` to `crates/worldwake-ai/tests/golden_perception_exposure.rs`. The golden scenario keeps two colocated agents on a reduced `observation_budget`, seeds two resource-source facilities plus heavy Waste clutter, and proves both agents still retain each other and both facilities while Waste retention stays bounded to the deterministic low-priority remainder.

Refreshed generated golden inventory outputs with `python3 scripts/golden_inventory.py --write --check-docs`. The current worktree already contained other local golden metadata changes, so the generator refresh produced mixed fallout: the S105-owned generated update is the new Scenario 341 entry and perception-exposure inventory surface, while broader inventory/index/detail churn also picked up pre-existing local metadata and source-line movement elsewhere in `docs/generated/`.

Clippy surfaced one test-only cleanup after the new golden landed. The observation summary carrier in `golden_perception_exposure.rs` was reshaped to avoid `clippy::struct_excessive_bools` without changing the asserted behavior.

## Deviations

1. The original ticket wording treated raw `observed_snapshots.len()` as the budget proof. The landed unit test instead counts observed entities excluding `batch.place`, because direct local observation stores the place snapshot separately from the budgeted entity set.
2. The original deterministic-order wording implied assertion on returned map order. The landed proof asserts that the retained same-priority Waste subset is the lowest-`EntityId` subset that fits under the budget, which is the real deterministic contract after truncation.
3. The golden proof surface was narrowed during reassessment. Instead of adding a separate survival-needs proof, the new golden focuses on the honest delta for S105: reduced-budget retention of agents and facilities with bounded Waste visibility.

## Verification Result

Passed on 2026-04-16:

1. `cargo test -p worldwake-systems --lib perception::tests::passive_local_observation_applies_budget_priority_to_non_place_entities -- --exact`
2. `cargo test -p worldwake-systems --lib perception::tests::passive_local_observation_boosts_non_waste_item_lots_when_needs_are_urgent -- --exact`
3. `cargo test -p worldwake-ai --test golden_perception_exposure golden_observation_budget_prioritizes_agents_and_facilities_over_waste -- --exact`
4. `cargo test -p worldwake-systems`
5. `cargo test -p worldwake-ai`
6. `python3 scripts/golden_inventory.py --write --check-docs`
7. `cargo clippy --workspace --all-targets -- -D warnings`
