# S95RELPLAHEU-005: Golden test extensions for FF heuristic

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: archive/tickets/S95RELPLAHEU-004.md

## Problem

After the FF heuristic is integrated into the search loop (ticket 004), existing golden tests exercise multi-step tactical search but do not assert that the FF heuristic is active and producing values. Without golden-level assertions, there is no E2E evidence that the RPG heuristic fires during realistic agent planning scenarios.

## Assumption Reassessment (2026-04-12)

1. `cross_location_water_acquisition_succeeds_without_budget_exhaustion` in `crates/worldwake-ai/tests/golden_planner_pathology.rs`, `landmark_depth_divergence` in `crates/worldwake-ai/tests/golden_reasoning_diversity.rs`, and `golden_remote_acquire_commodity_recipe_input` in `crates/worldwake-ai/tests/golden_production.rs` are all lawful planner goldens, but none currently exposes positive FF fields at the golden trace boundary. The real FF-positive E2E surface already live in `crates/worldwake-ai/tests/golden_care.rs` (`golden_healer_acquires_remote_ground_medicine_for_patient` via `assert_remote_care_tick_zero_plan`), which already inspects `expansion_summaries` for a successful remote multi-step search.
2. `SearchExpansionSummary` now has `ff_heuristic: Option<u32>` and `helpful_action_count: u16` after ticket 002, and ticket 004 has now landed the live search-path population for both fields.
3. Golden test infrastructure already captures `expansion_summaries` via the `search_plan` trace parameter — no new plumbing needed to access the FF fields.
4. Default `CognitiveProfile` now has `use_ff_heuristic: true` after ticket 001, so existing golden scenarios will automatically use the FF heuristic without scenario changes.

## Architecture Check

1. Golden tests assert observable trace properties of realistic scenarios. Adding `ff_heuristic.is_some()` and `helpful_action_count > 0` assertions to the existing remote-care planning-trace golden is the minimal change that provides truthful E2E validation of the FF heuristic pipeline.
2. No backward-compatibility shims. Assertions are additive to existing golden tests.

## Verification Layers

1. FF heuristic fires during realistic planning → golden test assertion on `ff_heuristic.is_some()` in expansion summaries
2. Helpful actions produced → golden test assertion on `helpful_action_count > 0`
3. Single-layer ticket — golden E2E only.

## What to Change

### 1. Extend the existing positive-FF planning-trace golden test

In the golden test that already exposes realistic expansion summaries for a successful remote multi-step tactical search, add assertions on the expansion summaries:

- At least one expansion summary has `ff_heuristic.is_some()` (RPG produced a value)
- At least one expansion summary has `helpful_action_count > 0` (helpful actions identified)

Target files:
- `crates/worldwake-ai/tests/golden_care.rs` — `assert_remote_care_tick_zero_plan` / `golden_healer_acquires_remote_ground_medicine_for_patient`

### 2. Regression guard

Add a comment documenting that these scenarios are intended as trace-level FF regression sentinels, not exact expansion-count thresholds. The heuristic-field assertions are the regression guard; do not add brittle count-based expectations unless the scenario is explicitly specified around that boundary.

## Files to Touch

- `crates/worldwake-ai/tests/golden_care.rs` (modify)

## Out of Scope

- New golden scenarios (extend existing ones only)
- Performance benchmarks or expansion-count regression thresholds (qualitative observation only)
- Unit or integration tests for the RPG algorithm (tickets 003, 004)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_healer_acquires_remote_ground_medicine_for_patient` passes with FF heuristic assertions
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. FF heuristic is active by default in all golden scenarios (CognitiveProfile defaults to `use_ff_heuristic: true`)
2. Expansion summaries contain non-None `ff_heuristic` values and positive `helpful_action_count` values for the chosen successful remote multi-step search

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_care.rs` — add FF trace assertions to the remote-care tick-0 planning helper

### Commands

1. `cargo test -p worldwake-ai --test golden_care golden_healer_acquires_remote_ground_medicine_for_patient -- --exact`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-04-12.

What changed: Added FF heuristic golden assertions to `crates/worldwake-ai/tests/golden_care.rs` inside `assert_remote_care_tick_zero_plan`, which is the live remote multi-step planning helper behind `golden_healer_acquires_remote_ground_medicine_for_patient`. The helper now asserts that at least one expansion summary exposes both a live `ff_heuristic` value and a positive `helpful_action_count`, and documents that these fields are the regression contract rather than any exact expansion-count threshold.

Deviations: The original ticket target set was corrected twice during reassessment. `cross_location_water_acquisition_succeeds_without_budget_exhaustion`, `landmark_depth_divergence`, and `golden_remote_acquire_commodity_recipe_input` are all lawful planner goldens, but none currently produces positive FF fields at the golden trace boundary, so extending them with `ff_heuristic.is_some()` / `helpful_action_count > 0` assertions would have been false. The implementation was narrowed to the single existing golden that truthfully exposes the new FF trace fields.

Verification results:
1. `cargo test -p worldwake-ai --test golden_care golden_healer_acquires_remote_ground_medicine_for_patient -- --exact`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
