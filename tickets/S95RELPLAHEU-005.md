# S95RELPLAHEU-005: Golden test extensions for FF heuristic

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S95RELPLAHEU-004

## Problem

After the FF heuristic is integrated into the search loop (ticket 004), existing golden tests exercise multi-step tactical search but do not assert that the FF heuristic is active and producing values. Without golden-level assertions, there is no E2E evidence that the RPG heuristic fires during realistic agent planning scenarios.

## Assumption Reassessment (2026-04-12)

1. Water-acquisition golden tests exist in `crates/worldwake-ai/tests/golden_planner_pathology.rs:486` (`cross_location_water_acquisition_succeeds_without_budget_exhaustion`) and in `golden_reasoning_diversity.rs:180,235` (AcquireCommodity(Water) scenarios). These exercise multi-step tactical search with expansion summaries.
2. `SearchExpansionSummary` will have `ff_heuristic: Option<u32>` and `helpful_action_count: u16` after ticket 002, populated with live values after ticket 004.
3. Golden test infrastructure already captures `expansion_summaries` via the `search_plan` trace parameter — no new plumbing needed to access the FF fields.
4. Default `CognitiveProfile` now has `use_ff_heuristic: true` after ticket 001, so existing golden scenarios will automatically use the FF heuristic without scenario changes.

## Architecture Check

1. Golden tests assert observable trace properties of realistic scenarios. Adding `ff_heuristic.is_some()` and `helpful_action_count > 0` assertions to existing water-acquisition tests is the minimal change that provides E2E validation of the FF heuristic pipeline.
2. No backward-compatibility shims. Assertions are additive to existing golden tests.

## Verification Layers

1. FF heuristic fires during realistic planning → golden test assertion on `ff_heuristic.is_some()` in expansion summaries
2. Helpful actions produced → golden test assertion on `helpful_action_count > 0`
3. Single-layer ticket — golden E2E only.

## What to Change

### 1. Extend water-acquisition golden tests

In the golden tests that exercise `AcquireCommodity(Water)` with multi-step tactical search, add assertions on the expansion summaries:

- At least one expansion summary has `ff_heuristic.is_some()` (RPG produced a value)
- At least one expansion summary has `helpful_action_count > 0` (helpful actions identified)

Target files:
- `crates/worldwake-ai/tests/golden_planner_pathology.rs` — `cross_location_water_acquisition_succeeds_without_budget_exhaustion`
- `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` — water-acquisition scenarios

### 2. Regression guard

Add a comment or assertion documenting that FF-enabled scenarios should produce fewer total expansions than the pre-FF budget-exhaustion baseline (qualitative — exact counts depend on scenario). This serves as a regression sentinel: if a future change breaks the RPG integration silently, the heuristic assertions will catch it before expansion counts regress.

## Files to Touch

- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify)
- `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` (modify)

## Out of Scope

- New golden scenarios (extend existing ones only)
- Performance benchmarks or expansion-count regression thresholds (qualitative observation only)
- Unit or integration tests for the RPG algorithm (tickets 003, 004)

## Acceptance Criteria

### Tests That Must Pass

1. `cross_location_water_acquisition_succeeds_without_budget_exhaustion` passes with FF heuristic assertions
2. Water-acquisition diversity scenarios pass with FF heuristic assertions
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. FF heuristic is active by default in all golden scenarios (CognitiveProfile defaults to `use_ff_heuristic: true`)
2. Expansion summaries contain non-None `ff_heuristic` values for multi-step tactical searches

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs` — add ff_heuristic assertions to water-acquisition test
2. `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` — add ff_heuristic assertions to water-acquisition scenarios

### Commands

1. `cargo test -p worldwake-ai -- golden_planner_pathology::cross_location_water`
2. `cargo test -p worldwake-ai -- golden_reasoning_diversity`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
