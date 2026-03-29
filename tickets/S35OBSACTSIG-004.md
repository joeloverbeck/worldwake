# S35OBSACTSIG-004: Extend `GoalBeliefView` with activity query methods

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-sim belief view trait
**Deps**: S35OBSACTSIG-001 (BelievedActivity type on BelievedEntityState)

## Problem

The AI ranking system needs to query observed activity from agent beliefs, but `GoalBeliefView` has no methods for this. This ticket adds `believed_activity_of()` and `agents_active_at()` to the trait, the macro, and the concrete implementations.

## Assumption Reassessment (2026-03-29)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:29` has ~30 methods for querying beliefs. No activity-related methods exist.
2. `impl_goal_belief_view!` macro at `belief_view.rs:517` delegates all `GoalBeliefView` methods to `RuntimeBeliefView` methods. Adding new `GoalBeliefView` methods requires adding corresponding `RuntimeBeliefView` methods.
3. `RuntimeBeliefView` trait is also in `belief_view.rs`. New methods must be added there too.
4. `PerAgentBeliefView` at `crates/worldwake-sim/src/per_agent_belief_view.rs` uses `impl_goal_belief_view!` — it will automatically get the new methods via the macro.
5. `OmniscientBeliefView` at `crates/worldwake-sim/src/omniscient_belief_view.rs` implements `GoalBeliefView` directly (or via the macro) — must also implement the new methods.
6. `BelievedEntityState` will have `believed_activity: Option<BelievedActivity>` after S35OBSACTSIG-001.
7. `known_entity_beliefs()` returns `Vec<(EntityId, BelievedEntityState)>` — the `agents_active_at()` implementation will iterate this to filter by place, domain, and target.

## Architecture Check

1. Adding query methods to `GoalBeliefView` follows the established pattern — the trait is the AI's window into beliefs. All existing belief queries follow this pattern.
2. `agents_active_at()` is a derived computation (iterates beliefs, filters) — never stored. Follows P25 (derived summaries are caches, never truth).
3. No backward compatibility shims — new trait methods added directly.

## Verification Layers

1. `believed_activity_of()` returns correct activity for observed agent -> focused unit test
2. `believed_activity_of()` returns `None` for unobserved agent -> focused unit test
3. `agents_active_at()` filters by place, domain, and target correctly -> focused unit test
4. `agents_active_at()` returns empty when no competitors match -> focused unit test
5. Single-layer ticket: belief view query layer only

## What to Change

### 1. Add methods to `GoalBeliefView` trait

In `crates/worldwake-sim/src/belief_view.rs`, add to the `GoalBeliefView` trait:

```rust
fn believed_activity_of(&self, entity: EntityId) -> Option<&BelievedActivity>;

fn agents_active_at(
    &self,
    place: EntityId,
    domain: ActionDomain,
    target: Option<EntityId>,
) -> Vec<EntityId>;
```

### 2. Add corresponding methods to `RuntimeBeliefView` trait

Add the same method signatures to `RuntimeBeliefView`.

### 3. Update `impl_goal_belief_view!` macro

Add delegation entries for both new methods.

### 4. Implement on `PerAgentBeliefView`

Implement `believed_activity_of()`: look up entity in belief store, return `believed_activity.as_ref()`.

Implement `agents_active_at()`: iterate `known_entity_beliefs`, filter for entities where `last_known_place == Some(place)` AND `believed_activity.map(|a| a.action_domain) == Some(domain)` AND (target is `None` OR `believed_activity.map(|a| a.target) == Some(target)`).

### 5. Implement on `OmniscientBeliefView`

`OmniscientBeliefView` is a stand-in. For `believed_activity_of()`: return `None` (omniscient view doesn't track activity beliefs). For `agents_active_at()`: return empty vec. These are safe defaults — omniscient view is used before belief system is fully active.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait + macro)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — implement new methods)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (modify — implement new methods)

## Out of Scope

- Perception logic that populates `BelievedActivity` (S35OBSACTSIG-003)
- Ranking discount that consumes these queries (S35OBSACTSIG-006)
- `BelievedActivity` type definition (S35OBSACTSIG-001, prerequisite)
- `PlanningState` / snapshot-level belief views — only runtime belief views are in scope

## Acceptance Criteria

### Tests That Must Pass

1. `believed_activity_of()` returns `Some(BelievedActivity)` for entity with activity set in belief store.
2. `believed_activity_of()` returns `None` for entity without activity.
3. `agents_active_at(place, Production, None)` returns all agents believed to be producing at that place.
4. `agents_active_at(place, Trade, Some(merchant))` returns only agents believed to be trading with that specific merchant.
5. `agents_active_at()` returns empty `Vec` when no agents match domain/place/target.
6. `agents_active_at()` does not include the querying agent itself (if the querying agent is at the same place with an active action — this depends on whether self is in the belief store, which it typically is not).
7. Existing suite: `cargo test --workspace`

### Invariants

1. `GoalBeliefView` methods are pure queries — they never mutate belief state.
2. `agents_active_at()` is a derived computation, never stored (P25).
3. Macro delegation correctly routes new methods to `RuntimeBeliefView` implementations.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (or adjacent test module) — focused tests using a manually constructed belief store with activity entries, verifying both query methods.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
