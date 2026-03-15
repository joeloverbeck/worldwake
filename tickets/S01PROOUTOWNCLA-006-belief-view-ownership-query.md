# S01PROOUTOWNCLA-006: Add believed_owner_of() to RuntimeBeliefView and PerAgentBeliefView

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — belief view trait and implementation
**Deps**: None (uses existing belief store and relation layer)

## Problem

The AI planner cannot reason about ownership when filtering pickup affordances. A `believed_owner_of()` method is needed on the `RuntimeBeliefView` trait so affordance filtering can distinguish "unowned lot I can freely pick up" from "someone else's lot I cannot lawfully take."

## Assumption Reassessment (2026-03-15)

1. `RuntimeBeliefView` trait at `belief_view.rs:109-229` already includes `can_control(actor, entity) -> bool` — confirmed
2. `PerAgentBeliefView` at `per_agent_belief_view.rs` wraps `AgentBeliefStore` + `World` — confirmed
3. `AgentBeliefStore` tracks believed relations including ownership — to verify exact storage
4. `OwnedBy` relation exists in `RelationKind` enum — confirmed
5. `believed_entity()` pattern exists on `PerAgentBeliefView` for querying belief store — confirmed

## Architecture Check

1. Adding one method to the trait + one implementation is minimal
2. Follows the same pattern as other belief queries (e.g., `entity_kind()`, `effective_place()`)
3. Returns `Option<EntityId>` — agents may not know the owner (belief gap), which is distinct from "unowned"
4. No world state reads — queries belief store only (Principle 10: belief-only planning)

## What to Change

### 1. Add trait method to `RuntimeBeliefView`

In `crates/worldwake-sim/src/belief_view.rs`:

```rust
fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId>;
```

### 2. Implement on `PerAgentBeliefView`

In `crates/worldwake-sim/src/per_agent_belief_view.rs`:

Query the agent's belief store for the `OwnedBy` relation on the given entity. Return `Some(owner)` if the agent believes the entity has an owner, `None` if the agent has no belief about ownership.

### 3. Implement on `OmniscientBeliefView` (if still exists as test helper)

If an omniscient belief view exists for testing, implement `believed_owner_of()` by directly querying `world.owner_of()`.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait method)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — implement method)
- Any other `RuntimeBeliefView` implementors (modify — add implementation)

## Out of Scope

- Using the method in affordance filtering (S01PROOUTOWNCLA-008)
- Belief propagation for ownership (ownership enters belief store via perception — already handled by E14)
- Institutional delegation in belief view (the belief-based `can_control()` changes are part of S01PROOUTOWNCLA-008)

## Acceptance Criteria

### Tests That Must Pass

1. `believed_owner_of()` returns `Some(owner)` when agent has seen ownership assignment
2. `believed_owner_of()` returns `None` when agent has no belief about entity's ownership
3. `believed_owner_of()` returns `None` for entities the agent hasn't perceived
4. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Belief-only access — never reads raw world state directly
2. Returns agent's belief, not ground truth (may be stale or absent)
3. All existing `RuntimeBeliefView` implementors compile with the new method

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` or `belief_view.rs` test module — believed ownership query tests

### Commands

1. `cargo test -p worldwake-sim believed_owner`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`
