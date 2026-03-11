# HARPREE14-015: Authoritative hypothetical action transitions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes -- AI search semantics, planning-state transition modeling
**Deps**: HARPREE14-010 (goal semantics extraction, archived), HARPREE14-011 (local crafted-output acquisition, archived)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-A01, Principle 3, Principle 12

## Problem

`search.rs` currently contains an ad hoc hypothetical state patch for `pick_up` so local ground lots can satisfy `AcquireCommodity`. That fix closed a real hole, but the shape is not durable:

1. Search now knows a transport-action special case by literal action name.
2. The hypothetical pickup transition can drift from the authoritative transport action implementation.
3. Future action families that need concrete hypothetical state transitions (partial pickup, put-down, staged craft inputs, looting, cargo moves) have no clean extension point.

This violates the hardening direction from HARDEN-A01: the search core should not accumulate action-family-specific logic.

## Assumption Reassessment (2026-03-12)

1. `apply_affordance_transition()` now exists in `crates/worldwake-ai/src/search.rs` and special-cases `def.name == "pick_up"` -- confirmed.
2. The authoritative pickup behavior lives in `crates/worldwake-systems/src/transport_actions.rs` and includes capacity-aware partial splits -- confirmed.
3. The current search-side transition is not derived from that authoritative transport behavior and can diverge from it -- confirmed.
4. The planner already has a semantics layer (`PlannerOpSemantics`) but it only classifies actions and barrier properties; it does not own hypothetical state deltas -- confirmed.

## Architecture Check

1. Hypothetical action transitions should be modeled as first-class planner semantics, not as search-local string matches. That keeps search generic and gives each action family one authoritative place to describe its hypothetical state effects.
2. A dedicated transition layer is cleaner than patching `search.rs` again for every new action family because it preserves Principle 12: state-transition knowledge belongs in planner semantics, not in the search algorithm.
3. The design must not introduce compatibility shims. The old ad hoc search helper should be removed once the transition layer exists.

## What to Change

### 1. Introduce planner-owned hypothetical transition semantics

Add an explicit planner transition surface owned by the AI semantics layer. One acceptable shape:

- extend `PlannerOpSemantics` with a transition discriminator, or
- add a dedicated `ActionTransitionSemantics` table keyed by `ActionDefId`, or
- add a trait/module that maps an affordance to a concrete hypothetical `PlanningState` delta

Requirements:

- search calls the transition surface generically
- transport-specific behavior is keyed by semantics/classification, not by action name strings
- unsupported actions fall back to the existing generic goal-model transition path

### 2. Make pickup transitions authoritative and capacity-correct

The hypothetical transition for pickup must mirror authoritative transport behavior:

- if the lot fully fits, move the whole lot into actor control
- if it does not fit, model the same split semantics as `execute_pick_up()`
- preserve deterministic lot/accounting outcomes in `PlanningState`

Do not implement this as a rough approximation; the planner must model the same concrete inventory consequences the real action would produce.

### 3. Define the boundary for future action-family transitions

Document and test how new families plug in. At minimum cover:

- pickup
- put-down
- a no-special-case action continuing to use generic goal-model updates

The goal is not to implement every future action now, but to leave a clean extensible seam so future hardening work does not reintroduce search-local hacks.

### 4. Remove the current ad hoc search special case

Delete the current direct `pick_up` name check from `search.rs` once the new transition surface is in place.

## Files to Touch

- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify if fallback transition ownership needs cleanup)
- `crates/worldwake-ai/src/lib.rs` (modify if new semantics module is exported)
- `crates/worldwake-ai/src/<new transition semantics module>.rs` (new, if needed)

## Out of Scope

- Recursive harvest -> craft multi-step planning across materialization barriers
- New action families unrelated to currently modeled transport/cargo semantics
- Commodity-balance changes such as making `Grain` non-edible
- Backward-compatibility wrappers around the old search helper

## Acceptance Criteria

### Tests That Must Pass

1. A search test proves local-lot acquisition uses the new semantics surface rather than a literal action-name special case.
2. A search/planning-state test proves partial-capacity pickup models the same split outcome as authoritative transport behavior.
3. A regression test proves a generic non-transport action still uses the fallback hypothetical transition path unchanged.
4. Existing targeted suites for search, planning state, transport actions, and golden e2e all pass.
5. Existing suite: `cargo test --workspace`
6. Existing lint: `cargo clippy --workspace`

### Invariants

1. Search remains generic and does not contain action-name-specific behavior.
2. Hypothetical inventory transitions remain concrete and capacity-aware, not abstract score updates.
3. No backward-compatibility aliasing or duplicate transition paths remain after the refactor.
4. Determinism is preserved for identical seeds and inputs.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` -- verify local pickup uses planner-owned transition semantics and still satisfies acquisition goals.
2. `crates/worldwake-ai/src/planning_state.rs` -- verify partial pickup / split outcomes are modeled concretely and deterministically.
3. `crates/worldwake-systems/src/transport_actions.rs` or existing transport tests -- cross-check authoritative semantics assumptions the planner transition mirrors.
4. `crates/worldwake-ai/tests/golden_e2e.rs` -- ensure the multi-recipe craft-path scenario still passes under the refactored transition layer.

### Commands

1. `cargo test -p worldwake-ai search`
2. `cargo test -p worldwake-ai planning_state`
3. `cargo test -p worldwake-systems transport_actions`
4. `cargo test -p worldwake-ai --test golden_e2e`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

