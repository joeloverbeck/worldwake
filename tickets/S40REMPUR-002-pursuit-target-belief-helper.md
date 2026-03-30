# S40REMPUR-002: Centralized pursuit_target_belief() helper

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Deps**: S40REMPUR-001 (PursuitProfile must exist)

## Problem

Multiple AI modules (candidate generation, goal-model place derivation, invalidation) will need to extract a target's believed remote location from belief state. Without a single shared helper, this logic would be duplicated across modules, violating DRY and making the provenance contract inconsistent.

## Assumption Reassessment (2026-03-30)

1. `BelievedEntityState` already has `last_known_place: Option<EntityId>`, `source: PerceptionSource`, `observed_tick: Tick` (`belief.rs:690-700`).
2. `AgentBeliefStore::known_entities` is a `BTreeMap<EntityId, BelievedEntityState>` (`belief.rs:40`).
3. `GoalBeliefView` trait is in `crates/worldwake-sim/src/belief_view.rs`. It exposes `known_entity_beliefs()` and `belief_confidence_policy()`.
4. `belief_confidence(source, staleness_ticks, policy) -> Permille` exists at `belief.rs:1168`.
5. `effective_place()` on `GoalBeliefView` returns the believed current place of an entity (`belief_view.rs`).
6. The helper must return `None` when: target place is unknown, target is believed dead, or target is already co-located with the actor. This ensures only genuinely remote targets trigger pursuit.
7. No adjacent contradictions exposed.

## Architecture Check

1. A single `pursuit_target_belief()` function is cleaner than inlining the extraction logic in candidate generation, invalidation, and goal-model. One source of truth for "what does the agent believe about where the target is?"
2. No backwards-compatibility shims.

## Verification Layers

1. Correct extraction from belief state → focused unit test with mock belief view
2. `None` when co-located → focused unit test
3. `None` when target place unknown → focused unit test
4. `None` when target believed dead → focused unit test
5. Provenance preserved → assert returned struct has correct `source` and `observed_tick`
6. Single-layer ticket (new helper function); no cross-system mapping needed.

## What to Change

### 1. Define `PursuitTargetBelief` struct and `pursuit_target_belief()` function

In a new module `crates/worldwake-ai/src/pursuit_belief.rs`:

```rust
pub struct PursuitTargetBelief {
    pub target: EntityId,
    pub believed_place: EntityId,
    pub source: PerceptionSource,
    pub observed_tick: Tick,
}

pub fn pursuit_target_belief(
    view: &dyn GoalBeliefView,
    actor: EntityId,
    target: EntityId,
) -> Option<PursuitTargetBelief> { ... }
```

Logic:
- Get `BelievedEntityState` for `target` from `view.known_entity_beliefs(actor)` (or equivalent getter)
- Return `None` if `alive == false`
- Return `None` if `last_known_place` is `None`
- Return `None` if `last_known_place == view.effective_place(actor)` (co-located, not remote)
- Otherwise return `Some(PursuitTargetBelief { ... })` with provenance fields

### 2. Re-export from `crates/worldwake-ai/src/lib.rs`

Make `pursuit_target_belief` and `PursuitTargetBelief` available to other modules within the AI crate.

## Files to Touch

- `crates/worldwake-ai/src/pursuit_belief.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify) — declare module

## Out of Scope

- Confidence derivation and threshold checks against `PursuitProfile` (that is candidate generation logic in S40REMPUR-004)
- Route cost checks (also S40REMPUR-004)
- Invalidation logic (S40REMPUR-005)
- Decision trace extensions (S40REMPUR-006)
- Any changes to `GoalBeliefView` trait or `BelievedEntityState` (the existing surface suffices)

## Acceptance Criteria

### Tests That Must Pass

1. `pursuit_target_belief()` returns `Some` with correct fields when target is believed alive at a remote place.
2. Returns `None` when target has no known place.
3. Returns `None` when target is believed dead.
4. Returns `None` when target is co-located with actor.
5. Returned `source` and `observed_tick` match the underlying `BelievedEntityState`.
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `PursuitTargetBelief` never stores a derived confidence value — it only carries provenance fields.
2. The function reads only from `GoalBeliefView`, never from authoritative world state.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/pursuit_belief.rs` (or `tests/` submodule) — focused unit tests for the four `None` cases and the `Some` case with provenance verification.

### Commands

1. `cargo test -p worldwake-ai pursuit_target_belief`
2. `cargo clippy -p worldwake-ai && cargo test -p worldwake-ai`
