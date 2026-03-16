# S01PROOUTOWNCLA-006: Add believed_owner_of() to RuntimeBeliefView and PerAgentBeliefView

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — belief view trait and implementation
**Deps**: None (uses existing belief store and relation layer)

## Problem

The AI planner cannot reason about ownership when filtering pickup affordances. A `believed_owner_of()` method is needed on the `RuntimeBeliefView` trait so affordance filtering can distinguish "unowned lot I can freely pick up" from "someone else's lot I cannot lawfully take."

## Assumption Reassessment (2026-03-15)

1. `RuntimeBeliefView` trait at `belief_view.rs:109-229` already includes `can_control(actor, entity) -> bool` — confirmed
2. `PerAgentBeliefView` at `per_agent_belief_view.rs` wraps `AgentBeliefStore` + `World` — confirmed
3. `AgentBeliefStore` tracks believed relations including ownership — **INCORRECT**: `BelievedEntityState` has no owner field; belief store does not track ownership. Implementation must follow the `direct_possessor()` pattern: gate on `knows_entity()`, then read `world.owner_of()`
4. `OwnedBy` relation exists in `relations.owned_by` BTreeMap — confirmed
5. `believed_entity()` pattern exists on `PerAgentBeliefView` for querying belief store — confirmed

## Architecture Check

1. Adding one method to the trait + one implementation is minimal
2. Follows the same pattern as other belief queries (e.g., `entity_kind()`, `effective_place()`)
3. Returns `Option<EntityId>` — agents may not know the owner (belief gap), which is distinct from "unowned"
4. Gates on belief awareness (`knows_entity()`), then reads `world.owner_of()` — follows the same pattern as `direct_possessor()` and `can_control()`. True belief-only ownership tracking deferred to E14 perception epic

## What to Change

### 1. Add trait method to `RuntimeBeliefView`

In `crates/worldwake-sim/src/belief_view.rs`:

```rust
fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId>;
```

### 2. Implement on `PerAgentBeliefView`

In `crates/worldwake-sim/src/per_agent_belief_view.rs`:

Gate on `knows_entity()` (or entity is owned by self), then query `self.world.owner_of(entity)`. Return `Some(owner)` if the agent is aware of the entity and it has an owner, `None` if the agent has no belief about the entity or the entity is unowned. Follows the exact `direct_possessor()` pattern.

### 3. Implement on all other `RuntimeBeliefView` implementors

`OmniscientBeliefView` does not exist. There are ~15 test stub implementations across worldwake-ai and worldwake-systems that must compile. All stubs should return `None`.

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

1. Belief-gated access — gates on `knows_entity()` before reading world state (same pattern as `direct_possessor()`)
2. Returns what the agent would believe based on awareness gating (may return `None` for entities the agent hasn't perceived)
3. All existing `RuntimeBeliefView` implementors compile with the new method

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` or `belief_view.rs` test module — believed ownership query tests

### Commands

1. `cargo test -p worldwake-sim believed_owner`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-16

**What changed**:
- Added `believed_owner_of(&self, entity: EntityId) -> Option<EntityId>` to `GoalBeliefView`, `RuntimeBeliefView`, and `impl_goal_belief_view!` macro in `belief_view.rs`
- Implemented on `PerAgentBeliefView`: gates on `knows_entity()` or self-ownership, then reads `world.owner_of()`
- Added `owner: Option<EntityId>` field to `SnapshotEntity` in `planning_snapshot.rs`, populated during snapshot build
- Implemented on `PlanningState`: reads from snapshot `owner` field
- Added `believed_owner_of` returning `None` to 16 test stub `RuntimeBeliefView` impls across worldwake-ai, worldwake-sim, and worldwake-systems

**Deviations from original plan**:
- Assumption #3 was wrong: `AgentBeliefStore` does not track ownership. Implementation follows the `direct_possessor()` pattern (belief-gated world reads) instead of querying the belief store directly. True belief-only ownership tracking deferred to E14.
- `OmniscientBeliefView` does not exist; section 3 was revised to cover the ~16 test stubs instead.
- Added `owner` field to `SnapshotEntity` + `PlanningState` impl (not mentioned in original ticket) to keep planning search accurate.

**Verification**:
- 4 new tests pass: `cargo test -p worldwake-sim believed_owner` (4/4)
- Full suite: `cargo test -p worldwake-sim` (287/287)
- `cargo clippy --workspace`: clean
