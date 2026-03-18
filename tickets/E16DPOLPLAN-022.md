# E16DPOLPLAN-022: Add `support_declarations_for_office` to `RuntimeBeliefView`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — belief_view.rs, omniscient_belief_view.rs, per_agent_belief_view.rs
**Deps**: None

## Problem

The GOAP planner cannot reason about whether an agent needs to build a coalition (Bribe/Threaten) before declaring support for an office claim, because the planning infrastructure has no access to existing support declarations for an office. The planner needs to know who supports whom so it can evaluate whether a self-declaration alone is sufficient or whether coalition-building steps are needed.

## Assumption Reassessment (2026-03-18)

1. `RuntimeBeliefView` already has `support_declaration(supporter, office) -> Option<EntityId>` for querying a single supporter's declaration — confirmed (belief_view.rs:102).
2. `World::support_declarations_for_office(office)` exists and returns `Vec<(EntityId, EntityId)>` — confirmed (world/social.rs:279).
3. `OmniscientBeliefView` and `PerAgentBeliefView` both implement `RuntimeBeliefView` — confirmed.
4. No existing method on `RuntimeBeliefView` enumerates ALL support declarations for an office — confirmed gap.

## Architecture Check

1. This follows the existing pattern: `support_declaration()` queries one supporter, the new method queries all declarations for an office. Both delegate to `World` methods that already exist. No new world state is introduced.
2. Principle 12 (World State ≠ Belief State): Under `PerAgentBeliefView`, the method should only return declarations the agent has observed. For now (pre-E14 full perception gating), both views delegate to `World` directly, matching the existing pattern for `support_declaration()`.
3. No backwards-compatibility shims. This is a new default trait method returning empty.

## What to Change

### 1. Add default method to `RuntimeBeliefView` trait (belief_view.rs)

```rust
fn support_declarations_for_office(&self, office: EntityId) -> Vec<(EntityId, EntityId)> {
    let _ = office;
    Vec::new()
}
```

Place after `support_declaration()` (line 105) for logical grouping.

Also add the same default method to the inner `RuntimeBeliefView` trait block (line ~226) used by `PerAgentBeliefView`.

### 2. Implement in `OmniscientBeliefView` (omniscient_belief_view.rs)

```rust
fn support_declarations_for_office(&self, office: EntityId) -> Vec<(EntityId, EntityId)> {
    self.world.support_declarations_for_office(office)
}
```

### 3. Implement in `PerAgentBeliefView` (per_agent_belief_view.rs)

```rust
fn support_declarations_for_office(&self, office: EntityId) -> Vec<(EntityId, EntityId)> {
    // Pre-E14: delegate to world directly, matching support_declaration() pattern.
    // Post-E14: gate by observation (agent must have perceived each declaration).
    self.world.support_declarations_for_office(office)
}
```

### 4. Add delegation in `impl_planning_belief_view!` macro (belief_view.rs)

Add the delegation entry in the macro alongside the existing `support_declaration` delegation.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait + macro)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (modify — impl)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — impl)

## Out of Scope

- Perception-gated filtering (post-E14)
- Changes to PlanningSnapshot or PlanningState (separate ticket)
- Changes to goal_model.rs or search.rs

## Acceptance Criteria

### Tests That Must Pass

1. `OmniscientBeliefView::support_declarations_for_office` returns correct declarations after world mutations
2. `PerAgentBeliefView::support_declarations_for_office` returns correct declarations
3. Both return empty `Vec` for offices with no declarations
4. Both return empty `Vec` for non-existent or dead entities
5. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Default trait implementation returns empty `Vec` (safe for TestBeliefView stubs)
2. Return type matches `World::support_declarations_for_office` exactly: `Vec<(supporter, candidate)>`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/omniscient_belief_view.rs` — test `support_declarations_for_office` returns declarations after `set_support_declaration`
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` — same pattern

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace`
