# S88TWOPHALAN-009: Golden tests for two-phase planning

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — new test file only
**Deps**: S88TWOPHALAN-007, S88TWOPHALAN-008, S88TWOPHALAN-010

## Problem

The two-phase planning architecture (S88) has no end-to-end golden tests proving that the system works as designed: multi-location plans are found, belief-only planning is respected, landmarks guide search correctly, agent diversity produces different profiles, and graceful degradation works. Without these tests, regressions in the planner's core behavioral contract are undetectable.

## Assumption Reassessment (2026-04-11)

1. Existing golden test files live in `crates/worldwake-ai/tests/golden_*.rs`. The new file `golden_two_phase_planning.rs` follows the same pattern. No existing file with this name.
2. Golden test setup patterns: existing golden tests (e.g., `golden_supply_chain.rs`, `golden_ai_decisions.rs`) build a `World` with `Topology`, create agents with explicit `CognitiveProfile` and `ExecutionBudget`, insert beliefs into `PlanningSnapshot`, and call `search_plan()` or run `agent_tick()`. The new tests follow the same harness.
3. The spec defines 6 scenarios (D10). Each exercises a distinct property of the two-phase system. Scenario 5 (regression guard) requires measuring branching factor, which is available via `SearchExpansionSummary.candidates_generated` in the decision trace.

## Architecture Check

1. Golden tests in a dedicated file follow the established pattern and are the correct verification surface for cross-component AI behavior.
2. No backwards-compatibility shims. This is entirely new test code.

## Verification Layers

1. Multi-location resource acquisition → golden test Scenario 1: strategic plan found, tactical plan at destination succeeds, expansion budget not exhausted
2. Belief-only planning (FND-14) → golden test Scenario 2: unknown locations excluded from plan
3. Landmark correctness → golden test Scenario 3: correct landmarks and orderings extracted
4. Agent cognitive diversity (FND-22) → golden test Scenario 4: different `landmark_extraction_depth` produces different search profiles
5. Branching factor regression guard → golden test Scenario 5: tactical candidate count < 100 with strategic decomposition vs > 1000 without
6. Graceful degradation → golden test Scenario 6: `landmark_extraction_depth = 0` still produces plans via spatial heuristic

## What to Change

### 1. Create `crates/worldwake-ai/tests/golden_two_phase_planning.rs`

**Scenario 1 — Multi-location resource acquisition**:
- Setup: Two places (A: barren, B: has Well + OrchardRow, 1 hop away). Agent at A with beliefs about B.
- Goal: AcquireCommodity(Water) or similar need-satisfaction goal.
- Assert: (a) strategic plan contains B as destination, (b) tactical plan at B finds action sequence, (c) plan found (not BudgetExhausted), (d) candidates_generated per expansion < 100.

**Scenario 2 — Belief-only planning (no omniscience)**:
- Setup: Agent at A. Location C exists (2 hops, has resource) but agent has NO beliefs about C.
- Assert: (a) strategic plan does NOT include C, (b) agent produces exploration or social query itinerary, (c) no plan to unknown resource found. Proves FND-14.

**Scenario 3 — Landmark correctness**:
- Setup: AcquireCommodity(Water) goal at location with Well.
- Assert: (a) landmarks include relevant facts (AtPlace, HasCommodity), (b) ordering is correct (AtPlace precedes HasCommodity), (c) preferred operators at depth 0 include Travel.
- Note: This may require exposing landmark extraction as a testable API or checking via decision trace enrichment.

**Scenario 4 — Agent cognitive diversity**:
- Setup: Two agents at same barren location, both with beliefs about remote resource. Agent X: `landmark_extraction_depth = 2`. Agent Y: `landmark_extraction_depth = 6`.
- Assert: Agents produce different expansion profiles (different expansion counts, different preferred candidate counts). Agent Y with deeper landmarks should find plans more efficiently.

**Scenario 5 — Regression guard (branching factor)**:
- Setup: Reproduce the high-candidate scenario from the simulation observer report (location with many entities/actions).
- Assert: With strategic decomposition active, tactical search produces < 100 candidates per expansion. Compare against same goal without strategic decomposition (candidates > 1000).

**Scenario 6 — Graceful degradation**:
- Setup: Agent with `landmark_extraction_depth = 0` at a location where the goal is locally satisfiable.
- Assert: Planner still functions using spatial heuristic only. Strategic decomposition still reduces candidates if multi-location.

### 2. Add perception profiles to test agents

Per CLAUDE.md: "Golden production tests require `PerceptionProfile` on agents that need to observe post-production output." Ensure all test agents have appropriate perception profiles for the scenarios.

## Files to Touch

- `crates/worldwake-ai/tests/golden_two_phase_planning.rs` (new)

## Out of Scope

- CLI-level integration tests
- Observer diagnostic tests (deferred per spec non-goals)
- Performance benchmarks with concrete timing thresholds (this is a correctness-focused spec, not a performance-optimization spec per se — the candidate count reduction is the metric, not wall-clock time)
- Modifying existing golden tests

## Acceptance Criteria

### Tests That Must Pass

1. `golden_multi_location_resource_acquisition` — plan found, candidates < 100
2. `golden_belief_only_planning_no_omniscience` — unknown locations excluded
3. `golden_landmark_correctness` — correct landmarks and orderings
4. `golden_agent_cognitive_diversity` — different depth produces different profiles
5. `golden_regression_guard_branching_factor` — candidates < 100 with strategic vs > 1000 without
6. `golden_graceful_degradation` — landmarks disabled, planner still works
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No golden test accesses world truth on behalf of an agent (FND-14)
2. All agents plan from beliefs seeded in the test setup
3. Deterministic: same seed produces same results (ChaCha8Rng)
4. Conservation is not violated by any test scenario

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_two_phase_planning.rs` — 6 golden scenarios per spec D10

### Commands

1. `cargo test -p worldwake-ai -- golden_two_phase`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test -p worldwake-ai`
3. `cargo test --workspace`
