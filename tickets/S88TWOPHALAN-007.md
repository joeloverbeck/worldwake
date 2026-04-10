# S88TWOPHALAN-007: Integrate two-phase planning into search_plan()

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — modifies planner search loop in worldwake-ai
**Deps**: S88TWOPHALAN-001, S88TWOPHALAN-002, S88TWOPHALAN-003, S88TWOPHALAN-004, S88TWOPHALAN-005, S88TWOPHALAN-006

## Problem

The individual components of two-phase planning (strategic planner, landmarks, dual frontier, landmark heuristic, profile fields) exist as isolated modules. This ticket wires them together in `search_plan()` to achieve the spec's core goal: reducing per-expansion candidate count from 1400+ to ~20–50 by decomposing multi-location goals into locality-scoped tactical sub-problems with landmark-guided search.

## Assumption Reassessment (2026-04-11)

1. `search_plan()` at `crates/worldwake-ai/src/search/mod.rs:79` currently uses a single `BinaryHeap<FrontierEntry>` (line 104), calls `search_candidates()` (line 143), `compute_heuristic()` via `combined_relevant_places()` (line 156–160), and applies beam truncation (line 282). The integration modifies this loop to: (a) run strategic planning before the loop, (b) extract landmarks for the tactical sub-goal, (c) replace `BinaryHeap` with `DualFrontier`, (d) compute preferred operators per expansion, (e) use combined spatial+landmark heuristic.
2. `CognitiveProfile` will have `landmark_extraction_depth` (S88TWOPHALAN-001). `ExecutionBudget` will have `preferred_operator_boost` (S88TWOPHALAN-002). Both are already passed to `search_plan()` at lines 85–86.
3. The existing `search_candidates()` function continues unchanged — candidate reduction comes from the strategic phase narrowing the sub-goal context, not from modifying candidate generation. The spec (D4) explicitly states: "No modifications to `search_candidates()` itself are required."
4. `build_successor_detailed` in `transition.rs` computes the heuristic per successor. The landmark heuristic integration adds a `.max(landmark_h)` to the existing spatial heuristic in the successor-building path.
5. `strategic::plan()` now takes `RecipeRegistry` in addition to `PlanningSnapshot`, `GroundedGoal`, and `ExecutionBudget`, because the live `goal_relevant_places()` / `prerequisite_places()` helpers require recipes for lawful goal-place derivation.
6. This is a planner-internal change (worldwake-ai only). No cross-system calls. Reads from `CognitiveProfile` and `ExecutionBudget` (core, read-only), `PlanningSnapshot` (AI-internal, read-only), and `RecipeRegistry`.

## Architecture Check

1. The integration follows the spec's layered approach: strategic phase runs once before the tactical loop, landmarks are extracted once per tactical call, and the dual frontier replaces the single heap. This is cleaner than interleaving strategic and tactical logic.
2. Graceful degradation: when `landmark_extraction_depth = 0`, no landmarks are extracted and the dual frontier operates with an empty preferred queue — equivalent to current single-heap behavior. When strategic planning returns no plan, the tactical search runs in local-only mode (current behavior).
3. No backwards-compatibility shims. The single `BinaryHeap` is replaced by `DualFrontier`.

## Verification Layers

1. Strategic plan drives travel target → integration test: multi-location goal produces strategic plan, tactical search finds plan at destination
2. Landmark-guided search reduces expansions → integration test: same goal with landmarks uses fewer expansions than without
3. Graceful degradation → integration test: `landmark_extraction_depth = 0` produces same behavior as current planner
4. Combined heuristic correctness → integration test: heuristic value is max(spatial, landmark)
5. Candidate count reduction → integration test: per-expansion candidates at tactical location < 100 (vs 1400+ for flat search)
6. Cross-layer: this is AI-internal, but the behavioral impact is observable through decision traces (S88TWOPHALAN-008) and golden tests (S88TWOPHALAN-009).

## What to Change

### 1. Convert SearchCandidate to PlanningOperator

Add a conversion function (either in `landmarks.rs` or a new helper in `mod.rs`) that maps the current `SearchCandidate` + `PlannerOpSemantics` into `PlanningOperator` (preconditions, add_effects, del_effects as `BTreeSet<PlanningFact>`). This bridges the existing candidate representation to the landmark system.

This conversion extracts facts from the candidate's preconditions and effects as represented in the action def semantics. The mapping is:
- Travel actions → `AtPlace` facts
- Acquisition actions → `HasCommodity`/`HasEntity` facts
- Consumption actions → `NeedSatisfied` facts
- Production/crafting → `HasCommodity` output facts

### 2. Modify `search_plan()` in `crates/worldwake-ai/src/search/mod.rs`

**Before the main loop** (after line 102):

```rust
// Phase 1: Strategic planning
let strategic_plan = strategic::plan(snapshot, goal, execution_budget, recipes);
// If strategic plan has steps, the first destination informs the tactical context.

// Phase 2: Extract landmarks for tactical search
let landmark_set = if cognitive.landmark_extraction_depth > 0 {
    let (initial_facts, goal_facts, operators) = /* extract from current state and goal */;
    landmarks::extract_landmarks(&initial_facts, &goal_facts, &operators, cognitive.landmark_extraction_depth)
} else {
    LandmarkSet::empty()
};

// Replace frontier
let mut frontier = DualFrontier::new(execution_budget.preferred_operator_boost);
```

**In the expansion loop** (after candidates are generated, around line 155):

```rust
// Compute preferred operators from landmarks
let preferred_indices = if !landmark_set.is_empty() {
    let current_facts = /* extract current planning facts from node.state */;
    landmarks::preferred_operators(&landmark_set, &current_facts, &candidates, &operators)
} else {
    BTreeSet::new()
};
```

**When inserting successors** (around line 282, after beam truncation):

```rust
for (i, (_, successor)) in successors.iter().enumerate() {
    let entry = FrontierEntry::new(successor.clone());
    if preferred_indices.contains(&i) {
        frontier.push_preferred(entry.clone());
    }
    frontier.push_regular(entry);
}
```

**Heuristic computation** (in `build_successor_detailed` or post-construction):

```rust
let spatial_h = compute_heuristic(snapshot, &successor.state, &combined_places.places);
let landmark_h = compute_landmark_heuristic(&landmark_set, &current_facts);
successor.heuristic_ticks = spatial_h.max(landmark_h);
```

### 3. Extract PlanningFact from PlanningState

Add a helper function to extract current `BTreeSet<PlanningFact>` from a `PlanningState`, mapping the agent's believed location, possessions, and need satisfaction status into `PlanningFact` values.

### 4. Write integration tests

Integration tests in `search/tests.rs` or a new `search/integration_tests.rs`:

- `test_strategic_plan_wired_into_search` — multi-location goal produces a found plan (not budget-exhausted)
- `test_landmarks_reduce_expansions` — same goal with `landmark_extraction_depth = 4` uses fewer expansions than `landmark_extraction_depth = 0`
- `test_graceful_degradation_no_landmarks` — `landmark_extraction_depth = 0` produces functionally equivalent results to current planner
- `test_dual_frontier_used_in_search` — preferred candidates are popped before regular candidates (observable via expansion order in traces)

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify — main integration)
- `crates/worldwake-ai/src/search/landmarks.rs` (modify — add SearchCandidate-to-PlanningOperator conversion if placed here)
- `crates/worldwake-ai/src/search/heuristic.rs` (modify — wire combined heuristic into successor building)
- `crates/worldwake-ai/src/search/transition.rs` (modify — if heuristic computation is done during successor building)
- `crates/worldwake-ai/src/search/tests.rs` (modify — add integration tests)

## Out of Scope

- Decision trace enrichment (S88TWOPHALAN-008)
- Golden E2E tests (S88TWOPHALAN-009)
- Modifying candidate generation pipeline (`search_candidates()`)
- Modifying action framework or world validation
- Observer diagnostic improvements (non-goal per spec)

## Acceptance Criteria

### Tests That Must Pass

1. All new integration tests for two-phase planning
2. All existing `search_plan` tests pass (graceful degradation)
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `search_plan()` with `landmark_extraction_depth = 0` and no strategic plan produces identical results to the pre-S88 planner
2. Strategic plan operates on beliefs only — never world truth (FND-14)
3. Landmarks are extracted from believed operators only (FND-14)
4. Per-expansion candidate count at tactical locations < 100 for multi-location goals that previously produced 1400+
5. All existing golden tests pass (no behavioral regression for goals that already work)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — integration tests for two-phase wiring
2. Possibly `crates/worldwake-ai/src/search/landmarks.rs` — SearchCandidate-to-PlanningOperator conversion tests

### Commands

1. `cargo test -p worldwake-ai -- search`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test -p worldwake-ai`
3. `cargo test --workspace`
