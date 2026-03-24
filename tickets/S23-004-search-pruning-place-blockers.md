# S23-004: Add blocker check to plan search for place-specific blockers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — search signature and pruning logic (worldwake-ai)
**Deps**: S23-001, S23-002, S23-003

## Problem

Place-specific blockers (SourceDepleted at Place A, WorkstationBusy at Place B) are no longer suppressing candidate generation (S23-003 makes candidate gen global-only). The plan search must now prune individual candidates at specific blocked locations, so agents route around blocked places to alternative sources. Without this, place-specific blockers have no effect at all.

## Assumption Reassessment (2026-03-24)

1. `search_plan()` at `search/mod.rs:72-82` currently takes: `snapshot, goal, semantics_table, registry, handlers, budget, recipes, binding_rejections, expansion_summaries` — confirmed. Needs new `blocked: &BlockedIntentMemory` parameter.
2. `search_candidates()` at `search/candidates.rs:93-101` currently takes: `goal, node, semantics_table, registry, handlers, binding_rejections, root_candidates` — confirmed. Needs new `blocked` and `current_tick` parameters.
3. Call sites of `search_plan()` — confirmed 4 production call sites:
   - `agent_tick/planning.rs:192`
   - `agent_tick/tests.rs:2445`
   - `search/tests.rs` (many test call sites)
   - `goal_model.rs` (4 test/validation call sites)
4. `SearchCandidate` struct at `candidates.rs:12-20` has `def_id`, `authoritative_targets`, `planning_targets` — used for place/target extraction.
5. `RootCandidateFilterReason` at `decision_trace.rs:253` currently has `BindingMismatch` and `BlockedFacilityUse` — needs new `PlaceBlocker` variant.
6. `PlanningState::effective_place_ref()` exists — confirmed, used for resolving actor's simulated place.
7. `PlannerOpKind::Travel` target is the destination place — confirmed from spec and existing search logic.
8. This is an AI-layer ticket at the planner search level. Existing search test harness is sufficient for focused tests; golden tests in S23-006 prove behavioral change.

## Architecture Check

1. Adding `blocked` to `search_plan()` follows the existing pattern of threading read-only references (like `budget`, `recipes`) through the search pipeline. The blocker memory is read-only during search.
2. `is_candidate_blocked()` runs AFTER binding check and BEFORE successor construction — minimal overhead for rejected candidates.
3. `PlaceBlocker` trace variant follows the existing `RootCandidateFilterReason` pattern for debuggability.
4. No backward-compatibility shims.

## Verification Layers

1. Candidate pruning by place-scoped blocker → focused unit test in search/tests.rs
2. Travel action uses destination as place → focused unit test
3. Non-travel action uses actor's current place → focused unit test
4. Trace records `PlaceBlocker` when candidate is pruned → focused unit test
5. Global blockers do NOT prune at search level (they already blocked at candidate gen) → focused unit test
6. End-to-end routing around blocked place → golden test in S23-006

## What to Change

### 1. `search_plan()` — add `blocked` parameter

```rust
pub fn search_plan(
    snapshot: &PlanningSnapshot,
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    budget: &PlanningBudget,
    recipes: &RecipeRegistry,
    blocked: &BlockedIntentMemory,  // NEW
    mut binding_rejections: Option<&mut Vec<BindingRejection>>,
    mut expansion_summaries: Option<&mut Vec<SearchExpansionSummary>>,
) -> PlanSearchResult
```

Thread `blocked` through to `search_candidates()` calls inside the search loop.

### 2. `search_candidates()` — add `blocked` and `current_tick`, implement pruning

After binding check passes and before pushing to filtered list, call `is_candidate_blocked()`.

### 3. Add `is_candidate_blocked()` helper

```rust
fn is_candidate_blocked(
    candidate: &SearchCandidate,
    goal: &GroundedGoal,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
) -> bool
```

Uses `candidate_action_place()` to resolve the place, then calls `blocked.is_blocked_for_search()`.

### 4. Add `candidate_action_place()` helper

```rust
fn candidate_action_place(
    candidate: &SearchCandidate,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> Option<EntityId>
```

Travel → target place from `authoritative_targets[0]`. All others → actor's simulated place from `effective_place_ref()`.

### 5. `decision_trace.rs` — add `PlaceBlocker` variant

```rust
// In RootCandidateFilterReason:
PlaceBlocker {
    place: Option<EntityId>,
    blocking_fact: BlockingFact,
},
```

### 6. Update all call sites

Pass `blocked` (or `&BlockedIntentMemory::default()` in tests that don't exercise blockers) to `search_plan()`:
- `agent_tick/planning.rs` — pass the agent's `BlockedIntentMemory` component
- `agent_tick/tests.rs` — pass `&BlockedIntentMemory::default()` or relevant test fixture
- `search/tests.rs` — pass `&BlockedIntentMemory::default()` for existing tests; add new blocker-specific tests
- `goal_model.rs` — pass `&BlockedIntentMemory::default()`

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify — `search_plan` signature, thread `blocked`)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — `search_candidates` signature, pruning logic, helpers)
- `crates/worldwake-ai/src/search/tests.rs` (modify — update all `search_plan` call sites, add blocker pruning tests)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — `PlaceBlocker` variant)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — pass `blocked` to `search_plan`)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — pass `blocked` to `search_plan`)
- `crates/worldwake-ai/src/goal_model.rs` (modify — pass `blocked` to `search_plan` in tests)

## Out of Scope

- **No changes to `blocked_intent.rs`** — that is S23-001
- **No changes to `failure_handling.rs`** — that is S23-002
- **No changes to `candidate_generation.rs`** — that is S23-003
- **No changes to `budget.rs`** — that is S23-005
- **No `UnknownBlockerTrace`** — that is S23-005
- **No golden test scenarios** — that is S23-006
- **Do not change search frontier/heuristic/transition logic** — only candidate filtering

## Acceptance Criteria

### Tests That Must Pass

1. All existing `search/tests.rs` tests pass with `&BlockedIntentMemory::default()` parameter
2. NEW: `place_scoped_blocker_prunes_candidate_at_blocked_place` — SourceDepleted at Place A prunes harvest at Place A
3. NEW: `place_scoped_blocker_does_not_prune_candidate_at_different_place` — SourceDepleted at Place A does NOT prune harvest at Place B
4. NEW: `travel_action_uses_destination_as_place` — Travel blocker at Place X prunes travel-to-X
5. NEW: `candidate_pruned_by_blocker_records_place_blocker_trace` — trace includes PlaceBlocker with correct place and fact
6. All existing `agent_tick/tests.rs` pass
7. All existing `goal_model.rs` tests pass
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `search_plan()` signature is backward-compatible for callers that pass `&BlockedIntentMemory::default()` — no behavior change without active blockers
2. Search pruning uses `is_blocked_for_search()` (no `blocks_goal_generation` gate) — ALL blocker types prune at search level
3. Deterministic pruning — BTreeMap iteration order is deterministic

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — all existing tests updated (default blocked param); 4+ new focused tests for place-scoped pruning, trace recording, travel vs non-travel place resolution
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — updated call sites only
3. `crates/worldwake-ai/src/goal_model.rs` — updated call sites only

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
