# S35OBSACTSIG-003: Implement `observe_active_actions()` perception helper

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-systems perception
**Deps**: S35OBSACTSIG-001 (BelievedActivity type, ActionDomain in core)

## Problem

`perception_system()` receives `active_actions` and `action_defs` via `SystemExecutionContext` but ignores both (destructured as `_active_actions`, `_action_defs`). Agents cannot observe what co-located agents are doing. This ticket adds the `observe_active_actions()` helper that populates `BelievedActivity` on `BelievedEntityState`.

## Assumption Reassessment (2026-03-29)

1. `perception_system()` at `crates/worldwake-systems/src/perception.rs:19` receives `SystemExecutionContext` which includes `active_actions: &BTreeMap<ActionInstanceId, ActionInstance>` and `action_defs: &ActionDefRegistry`. Both are currently unused (prefixed with `_`).
2. `observe_passive_local_entities()` at `perception.rs:176` receives `(world, event_log, tick, rng, updated_stores)` — does NOT receive active actions or action defs.
3. `ActionInstance` at `crates/worldwake-sim/src/action_instance.rs:6` has fields: `actor: EntityId`, `def_id: ActionDefId`, `targets: Vec<EntityId>`, among others.
4. `ActionDefRegistry` has `.get(def_id)` returning `Option<&ActionDef>`. `ActionDef` has a `domain: ActionDomain` field.
5. `PerceptionProfile` at `crates/worldwake-core/src/belief.rs:1243` has `observation_fidelity: Permille`.
6. The existing `passes_observation_check()` helper (or equivalent fidelity check) is used in `observe_passive_local_entities()` — must reuse the same pattern.
7. `BelievedEntityState` is stored in `AgentBeliefStore` keyed by `EntityId`.
8. After S35OBSACTSIG-001, `BelievedEntityState` will have `believed_activity: Option<BelievedActivity>`.

## Architecture Check

1. A separate helper function `observe_active_actions()` keeps the perception pipeline modular — one function per observation type (passive entities, event-based, active actions). Cleaner than inlining into `observe_passive_local_entities()`.
2. Called after `observe_passive_local_entities()` so entity beliefs already exist for co-located agents. Activity is layered on top.
3. Uses `observation_fidelity` gate per P20 — not all agents notice all activity.
4. Reads only `ActionInstance` + `ActionDef.domain` per P24 — no coupling to `ActionPayload` internals.
5. No backward compatibility shims.

## Verification Layers

1. Activity observed when co-located + fidelity passes -> focused test (construct scenario, assert belief)
2. Activity NOT observed when fidelity fails -> focused test (set fidelity to 0, assert None)
3. Activity cleared when observed agent idle -> focused test (no active action, assert None)
4. Activity cleared when agent departs -> focused test (move agent, re-run perception, assert None)
5. Activity observation is local only (not cross-place) -> focused test (different places, assert None)

## What to Change

### 1. Remove `_` prefix from `active_actions` and `action_defs` in `perception_system()`

In `crates/worldwake-systems/src/perception.rs`, the destructuring of `SystemExecutionContext` currently uses `_active_actions` and `_action_defs`. Remove the underscore prefixes.

### 2. Add `observe_active_actions()` function

```rust
fn observe_active_actions(
    world: &World,
    tick: Tick,
    rng: &mut DeterministicRng,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    updated_stores: &mut BTreeMap<EntityId, AgentBeliefStore>,
)
```

Logic per spec Section 3:
- For each agent with `PerceptionProfile` and a belief store in `updated_stores`:
  - Determine agent's place.
  - Build set of co-located actors with active actions.
  - For each co-located actor (not self): fidelity check -> set `believed_activity`.
  - For co-located actors with NO active action: set `believed_activity = None`.

### 3. Call from `perception_system()`

Insert call to `observe_active_actions()` after `observe_passive_local_entities()` returns.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify — add function, integrate into pipeline)

## Out of Scope

- `GoalBeliefView` extensions (S35OBSACTSIG-004)
- Ranking discount (S35OBSACTSIG-006)
- `BelievedActivity` type definition (S35OBSACTSIG-001, prerequisite)
- `UtilityProfile` changes (S35OBSACTSIG-002)
- Golden tests (S35OBSACTSIG-007)
- Modifying `observe_passive_local_entities()` — activity observation is a separate concern

## Acceptance Criteria

### Tests That Must Pass

1. `BelievedActivity` set when co-located agent has active action and fidelity check passes.
2. `BelievedActivity` not set when fidelity check fails (fidelity = `Permille(0)`).
3. `BelievedActivity` cleared when observed agent has no active action.
4. `BelievedActivity` cleared when observed agent departs (no longer co-located).
5. Activity observation does not cross place boundaries.
6. Observer does not observe their own activity.
7. Existing suite: `cargo test --workspace`

### Invariants

1. `observe_active_actions()` only reads `ActionDef.domain` and `ActionInstance.targets.first()` — no coupling to `ActionPayload` (P24).
2. Observation is gated by `observation_fidelity` (P20).
3. Only co-located agents are observable (P7 — locality).
4. `believed_activity` is only modified through perception, never directly by other systems.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` (or `tests/` module) — focused tests for each acceptance criterion above. Tests will construct a minimal world with two agents at the same place, one with an active action, and verify belief state.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
