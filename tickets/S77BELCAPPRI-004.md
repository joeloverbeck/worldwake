# S77BELCAPPRI-004: Remove SceneEvidence gate on place observation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — perception place observation gate removed
**Deps**: None

## Problem

`should_observe_current_place_entity()` returns `true` only if the place has a `SceneEvidence` component or the agent already has an evidence belief about it. Agents at evidence-free places never form beliefs about the place itself, which breaks the planner's ability to reason about the place graph. An agent standing at a location should always observe the place entity.

## Assumption Reassessment (2026-04-09)

1. `should_observe_current_place_entity()` at `perception.rs:484-494`. Current logic: `world.get_component_scene_evidence(place).is_some() || store.get_entity(&place).and_then(|belief| belief.believed_evidence.as_ref()).is_some()`. Single call site at `perception.rs:447`.
2. `SceneEvidence` defined at `crates/worldwake-core/src/evidence.rs:57`. Used in perception gating and evidence decay system.
3. Removing the gate does not affect `SceneEvidence` itself or its decay system — those remain for their own purposes (evidence-bearing places). This change only removes the condition that gates place observation on evidence presence.

## Architecture Check

1. Removing the gate aligns with P7 (Locality) and P15 (Knowledge Acquired Locally) — an agent physically present at a place should perceive it. The SceneEvidence gate was an over-restriction that prevented agents from learning about evidence-free locations.
2. No backward-compatibility shims. The gate function is either simplified to return `true` or removed entirely, with the call site unconditionally proceeding.

## Verification Layers

1. Agents observe their current place regardless of SceneEvidence -> focused unit test placing agent at evidence-free place, confirming observation occurs
2. Agents at evidence-bearing places still observe (no regression) -> existing perception tests cover this path
3. Single-layer ticket: perception system internal change, no cross-system interaction

## What to Change

### 1. Remove or simplify `should_observe_current_place_entity()`

In `crates/worldwake-systems/src/perception.rs`, the function at line 484 can be either:

**(a) Simplified** to always return `true`:
```rust
fn should_observe_current_place_entity(_world: &World, _place: EntityId, _store: &AgentBeliefStore) -> bool {
    true
}
```

**(b) Removed entirely**, with the call site at line 447 changed from:
```rust
if should_observe_current_place_entity(world, place, store)
    && passes_observation_check(observation_fidelity, rng)
```
to:
```rust
if passes_observation_check(observation_fidelity, rng)
```

Option (b) is preferred — dead code should be removed, not commented out or simplified to a constant.

### 2. Verify call site

The single call site at `perception.rs:447` is inside `observe_passive_local_entities()`. After the gate removal, the place observation block (lines 447-453) proceeds unconditionally (subject only to `passes_observation_check`), meaning agents always attempt to observe the place entity they occupy.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify)

## Out of Scope

- Changing `SceneEvidence` itself or its decay system
- Changing perception of non-place entities
- Modifying belief capacity or eviction logic (separate tickets)
- Changing `passes_observation_check` or observation fidelity

## Acceptance Criteria

### Tests That Must Pass

1. New: Agent at a place without `SceneEvidence` observes the place entity (belief about place appears in `known_entities`)
2. Existing: Agent at a place with `SceneEvidence` still observes the place (no regression)
3. Existing suite: `cargo test -p worldwake-systems -- perception`

### Invariants

1. An agent always forms beliefs about the place entity they currently occupy (bounded by `passes_observation_check` and capacity limits)
2. `SceneEvidence` continues to function for its own purpose (evidence decay) — this ticket only removes the observation gate

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` — `agent_observes_place_without_scene_evidence` — place observation no longer requires SceneEvidence

### Commands

1. `cargo test -p worldwake-systems -- perception`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`
