# S88TWOPHALAN-003: Implement PlanningFact and landmark extraction

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — new module internal to worldwake-ai planner
**Deps**: None

## Problem

The planner has no subgoal-structure guidance. It treats all candidates equally, causing budget exhaustion at depth 0 when branching factor is 1400+. Landmark extraction (S88 D5) derives mandatory intermediate milestones from the goal and available operators, enabling preferred-operator guidance that focuses search on landmark-achieving actions.

## Assumption Reassessment (2026-04-11)

1. `crates/worldwake-ai/src/search/` contains `mod.rs`, `frontier.rs`, `heuristic.rs`, `transition.rs`, `candidates.rs`, `tests.rs`. The new `landmarks.rs` module will be added alongside these. No existing `landmarks.rs` file.
2. The spec's `PlanningFact` enum references `EntityId` (from `worldwake-core`), `CommodityKind` (from `worldwake-core`), and `HomeostaticNeedId` (from `worldwake-core`). All exist and are importable from `worldwake-ai`.
3. This is a self-contained new module. The shared boundary is the `PlanningFact` and `LandmarkSet` types, which will be consumed by S88TWOPHALAN-005 (heuristic) and S88TWOPHALAN-007 (integration).

## Architecture Check

1. A standalone module with pure functions operating on `BTreeSet<PlanningFact>` and `PlanningOperator` slices is the cleanest design — no mutation of shared state, no cross-system calls. The module is testable in isolation.
2. No backwards-compatibility shims. This is entirely new code.

## Verification Layers

1. Landmark extraction correctness → focused unit tests in `landmarks.rs` (goal facts become landmarks, shared preconditions are discovered, ordering is correct)
2. Preferred operator derivation → focused unit tests (operators achieving next-actionable landmarks are marked preferred)
3. Empty/degenerate cases → focused unit tests (no achievers, empty operators, all goals initially true)
4. Single-layer ticket (new AI-internal module) — no cross-layer mapping needed.

## What to Change

### 1. Create `crates/worldwake-ai/src/search/landmarks.rs`

Implement the types and functions from S88 D5:

**Types**:
- `PlanningFact` enum: `AtPlace(EntityId)`, `HasCommodity(CommodityKind)`, `HasEntity(EntityId)`, `FacilityAvailable(EntityId)`, `EntityPresent(EntityId)`, `NeedSatisfied(HomeostaticNeedId)`. Derive `Clone, Ord, PartialOrd, Eq, PartialEq, Debug`.
- `PlanningOperator` struct: `preconditions: BTreeSet<PlanningFact>`, `add_effects: BTreeSet<PlanningFact>`, `del_effects: BTreeSet<PlanningFact>`.
- `LandmarkSet` struct: `landmarks: BTreeSet<PlanningFact>`, `orderings: Vec<(PlanningFact, PlanningFact)>`. Include `LandmarkSet::empty()` constructor.

**Functions**:
- `extract_landmarks(initial_facts, goal_facts, operators, max_depth) -> LandmarkSet` — delete-relaxation landmark extraction per spec algorithm.
- `preferred_operators(landmarks, current_facts, candidates, operators) -> BTreeSet<usize>` — returns indices of candidates whose operators achieve actionable landmarks.

### 2. Register module in `crates/worldwake-ai/src/search/mod.rs`

Add `pub(crate) mod landmarks;` to the module declarations.

### 3. Write focused unit tests

Tests within `landmarks.rs` or in a `landmarks/tests.rs`:

- `test_goal_facts_are_landmarks` — goal facts appear in extracted landmarks
- `test_shared_precondition_discovery` — when all achievers of a goal fact share a precondition, that precondition becomes a landmark with ordering
- `test_no_achievers_marks_unachievable` — facts with no achievers are handled gracefully
- `test_initial_facts_skipped` — facts already true in initial state are not processed
- `test_max_depth_limits_chain` — extraction stops at `max_depth`
- `test_empty_operators_returns_goal_landmarks` — with no operators, only goal facts are landmarks
- `test_preferred_operators_selects_landmark_achievers` — operators achieving next-actionable landmarks are preferred
- `test_preferred_operators_empty_when_no_landmarks` — empty LandmarkSet produces no preferred operators

## Files to Touch

- `crates/worldwake-ai/src/search/landmarks.rs` (new)
- `crates/worldwake-ai/src/search/mod.rs` (modify — add module declaration)

## Out of Scope

- Integration with `search_plan()` loop (S88TWOPHALAN-007)
- Landmark count heuristic (S88TWOPHALAN-005)
- Converting actual `SearchCandidate` objects into `PlanningOperator` form (S88TWOPHALAN-007)
- Strategic planner (S88TWOPHALAN-006)

## Acceptance Criteria

### Tests That Must Pass

1. All 8+ focused unit tests for landmark extraction and preferred operator derivation
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Goal facts are always included in the landmark set
2. Landmark orderings are acyclic (predecessor must be achievable before successor)
3. `extract_landmarks` with `max_depth = 0` returns only goal facts
4. `preferred_operators` returns empty set when `LandmarkSet::empty()` is used

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/landmarks.rs` (inline tests) — correctness of extraction algorithm, ordering, preferred operators, edge cases

### Commands

1. `cargo test -p worldwake-ai -- landmarks`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

Completed on 2026-04-11.

- Added [`search/landmarks.rs`] with `PlanningFact`, `PlanningOperator`,
  `LandmarkSet`, `extract_landmarks`, and `preferred_operators`, plus 8 focused
  inline unit tests covering landmark extraction, ordering, depth limits, and
  preferred-operator selection.
- Registered the module in `search/mod.rs` for downstream S88 tickets.
- Marked the new module with `#![allow(dead_code)]` because this ticket lands
  staged planner scaffolding that is intentionally unused until S88TWOPHALAN-005
  and S88TWOPHALAN-007 integrate it.

## Verification Result

- Passed `cargo test -p worldwake-ai -- landmarks`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
