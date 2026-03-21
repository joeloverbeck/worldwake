# S12PLAPREAWA-006: Unit tests for prerequisite-aware search

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only
**Deps**: S12PLAPREAWA-001, S12PLAPREAWA-002, S12PLAPREAWA-003, S12PLAPREAWA-004, S12PLAPREAWA-005

## Problem

The prerequisite-aware search enhancement (S12PLAPREAWA-001 through 005) introduces new trait methods, a new search helper, and modified heuristic behavior. Each component needs focused unit-level coverage to verify correctness in isolation before integration via golden tests.

## Assumption Reassessment (2026-03-21)

1. `prerequisite_places()` exists on `GoalKindPlannerExt` after S12PLAPREAWA-002 — assumed.
2. `combined_relevant_places()` exists as a private function in `search.rs` after S12PLAPREAWA-003 — assumed.
3. `PlanningBudget` has `max_prerequisite_locations: u8` after S12PLAPREAWA-001 — assumed.
4. `SearchExpansionSummary` has `combined_places_count` and `prerequisite_places_count` after S12PLAPREAWA-005 — assumed.
5. Existing test infrastructure in `goal_model.rs` and `search.rs` uses `PlanningSnapshot`, `PlanningState`, `RecipeRegistry`, and related builders — confirmed via existing unit tests in those modules.
6. Test-only ticket — no production code changes, no AI regression concerns.

## Architecture Check

1. Unit tests in the same modules as the code they test follows the existing project pattern (test modules at bottom of source files).
2. No backwards-compatibility concerns — pure test additions.

## Verification Layers

1. Each test verifies one specific behavior → test naming and assertions
2. All tests pass → `cargo test -p worldwake-ai`
3. Single-layer ticket — test-only.

## What to Change

### 1. `goal_model.rs` unit tests (7 tests)

Add to the existing `#[cfg(test)] mod tests` block:

1. **`prerequisite_places_treat_wounds_without_medicine`**: Set up a `PlanningSnapshot` with an agent lacking Medicine, a resource source for Medicine at Place B. Assert `prerequisite_places()` returns `[Place_B]`.

2. **`prerequisite_places_treat_wounds_with_medicine`**: Same setup but agent has Medicine in inventory. Assert `prerequisite_places()` returns `[]`.

3. **`prerequisite_places_treat_wounds_seller`**: Agent lacks Medicine, a merchant selling Medicine at Place C. Assert `prerequisite_places()` returns `[Place_C]`.

4. **`prerequisite_places_produce_commodity_missing_input`**: Recipe requires Wheat, agent lacks Wheat, Wheat resource source at Place D. Assert `prerequisite_places()` returns `[Place_D]`.

5. **`prerequisite_places_produce_commodity_has_all_inputs`**: Agent has all recipe inputs. Assert `prerequisite_places()` returns `[]`.

6. **`prerequisite_places_capped_by_budget`**: Agent knows Medicine exists at 5 places. With `max_prerequisite_locations: 3`, assert returns exactly 3 closest by travel distance.

7. **`all_goal_kind_variants_have_prerequisite_places_impl`**: Exhaustive match coverage test — call `prerequisite_places()` for every `GoalKind` variant. Mirrors existing `all_goal_kind_variants_have_goal_relevant_places_impl`.

### 2. `search.rs` unit tests (4 tests)

Add to the existing `#[cfg(test)] mod tests` block:

8. **`combined_places_includes_prerequisites_when_lacking`**: Set up TreatWounds scenario — agent at Place A (patient location), Medicine source at Place B. Assert `combined_relevant_places()` returns both Place A and Place B.

9. **`combined_places_excludes_prerequisites_after_hypothetical_pickup`**: Same setup, but after `apply_pick_up_transition()` updates `PlanningState`. Assert combined set no longer includes Place B.

10. **`pruning_retains_travel_to_prerequisite_location`**: Agent at Place A (= patient location), medicine at Place B. Run `prune_travel_away_from_goal()` with combined places. Assert travel to Place B is NOT pruned.

11. **`heuristic_guides_toward_prerequisite_when_lacking`**: Agent at Place A, patient at Place A, medicine at Place B. Assert heuristic for a node at Place B is lower than for a node at Place A (when prerequisites are unsatisfied).

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — add tests to existing test module)
- `crates/worldwake-ai/src/search.rs` (modify — add tests to existing test module)

## Out of Scope

- Production code changes — this ticket is test-only
- Golden tests (S12PLAPREAWA-007)
- Any changes outside test modules
- Test infrastructure/harness changes (use existing builders and helpers)

## Acceptance Criteria

### Tests That Must Pass

1. All 11 new unit tests listed above pass
2. Existing suite: `cargo test -p worldwake-ai`
3. `cargo clippy --workspace` — no warnings

### Invariants

1. No production code is modified
2. Tests use only the public API of the modules under test (or `pub(crate)` where necessary for `combined_relevant_places`)
3. Tests follow existing project naming conventions and structure

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — 7 new unit tests for `prerequisite_places()`
2. `crates/worldwake-ai/src/search.rs` — 4 new unit tests for combined places, pruning, and heuristic behavior

### Commands

1. `cargo test -p worldwake-ai goal_model`
2. `cargo test -p worldwake-ai search`
3. `cargo test -p worldwake-ai && cargo clippy --workspace`
