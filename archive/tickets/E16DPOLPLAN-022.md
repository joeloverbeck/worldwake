# E16DPOLPLAN-022: Add `support_declarations_for_office` to `RuntimeBeliefView`

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — belief_view.rs, omniscient_belief_view.rs, per_agent_belief_view.rs
**Deps**: None

## Problem

The GOAP planner cannot reason about whether an agent needs to build a coalition (Bribe/Threaten) before declaring support for an office claim, because the planning infrastructure has no access to existing support declarations for an office. The planner needs to know who supports whom so it can evaluate whether a self-declaration alone is sufficient or whether coalition-building steps are needed.

## Assumption Reassessment (2026-03-18, corrected)

1. Both `GoalBeliefView` (belief_view.rs:102) and `RuntimeBeliefView` (belief_view.rs:226) have `support_declaration(supporter, office) -> Option<EntityId>` as default methods — confirmed.
2. `World::support_declarations_for_office(office)` exists and returns `Vec<(EntityId, EntityId)>` — confirmed (world/social.rs:279).
3. **CORRECTED**: `OmniscientBeliefView` does NOT exist. The belief view architecture has `GoalBeliefView` (trait), `RuntimeBeliefView` (trait), `PerAgentBeliefView` (concrete impl), and `PlanningState` (concrete impl in worldwake-ai). The `impl_goal_belief_view!` macro delegates `GoalBeliefView` to `RuntimeBeliefView`.
4. No existing method on either trait enumerates ALL support declarations for an office — confirmed gap.

## Architecture Check

1. This follows the existing pattern: `support_declaration()` queries one supporter, the new method queries all declarations for an office. Both delegate to `World` methods that already exist. No new world state is introduced.
2. Principle 12 (World State ≠ Belief State): Under `PerAgentBeliefView`, the method should only return declarations the agent has observed. For now (pre-E14 full perception gating), the view delegates to `World` directly, matching the existing pattern for `support_declaration()`.
3. No backwards-compatibility shims. This is a new default trait method returning empty.

## What to Change

### 1. Add default method to `GoalBeliefView` trait (belief_view.rs)

```rust
fn support_declarations_for_office(&self, office: EntityId) -> Vec<(EntityId, EntityId)> {
    let _ = office;
    Vec::new()
}
```

Place after `support_declaration()` (line 105) for logical grouping.

### 2. Add default method to `RuntimeBeliefView` trait (belief_view.rs)

```rust
fn support_declarations_for_office(&self, office: EntityId) -> Vec<(EntityId, EntityId)> {
    let _ = office;
    Vec::new()
}
```

Place after `support_declaration()` (line 229) for logical grouping.

### 3. Add delegation in `impl_goal_belief_view!` macro (belief_view.rs)

Add delegation entry after the existing `support_declaration` delegation (~line 563):

```rust
fn support_declarations_for_office(&self, office: worldwake_core::EntityId) -> Vec<(worldwake_core::EntityId, worldwake_core::EntityId)> {
    $crate::RuntimeBeliefView::support_declarations_for_office(self, office)
}
```

### 4. Implement in `PerAgentBeliefView` (per_agent_belief_view.rs)

```rust
fn support_declarations_for_office(&self, office: EntityId) -> Vec<(EntityId, EntityId)> {
    // Pre-E14: delegate to world directly, matching support_declaration() pattern.
    // Post-E14: gate by observation (agent must have perceived each declaration).
    self.world.support_declarations_for_office(office)
}
```

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — both traits + macro)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — impl)

## Out of Scope

- Perception-gated filtering (post-E14)
- Changes to PlanningSnapshot or PlanningState (separate ticket)
- Changes to goal_model.rs or search.rs

## Acceptance Criteria

### Tests That Must Pass

1. `PerAgentBeliefView::support_declarations_for_office` returns correct declarations after world mutations
2. Returns empty `Vec` for offices with no declarations
3. Returns empty `Vec` for non-existent or dead entities
4. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Default trait implementation returns empty `Vec` (safe for TestBeliefView stubs)
2. Return type matches `World::support_declarations_for_office` exactly: `Vec<(supporter, candidate)>`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — test `support_declarations_for_office` returns declarations after `set_support_declaration`, returns empty for no declarations and non-existent entities

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-18
- **What changed**:
  - Added `support_declarations_for_office` default method to `GoalBeliefView` and `RuntimeBeliefView` traits (returns empty `Vec`)
  - Added delegation in `impl_goal_belief_view!` macro
  - Implemented in `PerAgentBeliefView` delegating to `World::support_declarations_for_office`
  - Added 3 tests: multi-supporter declarations, empty office, non-office entity
- **Deviations from original plan**: Ticket originally referenced `OmniscientBeliefView` and `omniscient_belief_view.rs`, which do not exist. Corrected to target both `GoalBeliefView` and `RuntimeBeliefView` traits plus the `impl_goal_belief_view!` macro delegation. Only `PerAgentBeliefView` needed a concrete impl.
- **Verification**: 308 sim tests pass, clippy clean, full workspace green
