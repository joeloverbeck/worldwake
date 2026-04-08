# S75BELVDECOM-008: GoalBeliefView decomposition + macro update + mock cleanup

**Status**: PENDING
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: Yes — GoalBeliefView trait, impl_goal_belief_view! macro, test mock ergonomics
**Deps**: S75BELVDECOM-007

## Problem

After all RuntimeBeliefView sub-traits are extracted and SnapshotEntity is decomposed, GoalBeliefView (92 methods) still remains monolithic even though `impl_goal_belief_view!` has already begun delegating some methods through the new RuntimeBeliefView sub-traits. This ticket finishes that decomposition by making GoalBeliefView a supertrait of planning-relevant sub-traits, completing the delegation cleanup, and adding mock helper infrastructure so test mocks can implement only the sub-traits they need.

## Assumption Reassessment (2026-04-08)

1. `GoalBeliefView` confirmed at `belief_view.rs:34` with 92 methods. It is a separate trait (not a sub-trait of RuntimeBeliefView).
2. `impl_goal_belief_view!` macro confirmed at `belief_view.rs:754`. Used by `PerAgentBeliefView` and `PlanningState` to mechanically delegate GoalBeliefView reads. After `S75BELVDECOM-002`, some delegations already route through `EntityBeliefView` / `ProfileBeliefView`, so the remaining work is to finish trait-level decomposition rather than to introduce the first sub-trait-aware delegations.
3. Test mocks across AI/sim/systems now already implement split RuntimeBeliefView sub-traits after `S75BELVDECOM-002`, but they still carry repetitive boilerplate. Mock helper infrastructure remains a valid cleanup target for the remaining GoalBeliefView-facing surfaces.

## Architecture Check

1. GoalBeliefView becomes a supertrait composing the planning-relevant sub-traits. Since GoalBeliefView excludes some RuntimeBeliefView methods (21 of 113), it won't compose all 11 sub-traits — likely excluding portions of TemporalBeliefView and FacilityBeliefView. The exact composition is determined by auditing which of GoalBeliefView's 92 methods map to which sub-traits.
2. The macro update is mechanical — grouping delegations by sub-trait instead of as a flat list.
3. Mock helpers use Rust's default method implementations on sub-traits (returning `unimplemented!()` or sensible defaults) so mocks only need to override the methods their test exercises.

## Verification Layers

1. GoalBeliefView composition -> `cargo build --workspace` (compile-time proof)
2. Macro delegation correctness -> all golden tests pass (they exercise GoalBeliefView through planning)
3. Mock ergonomics -> test compilation succeeds with simplified mock implementations

## What to Change

### 1. Audit GoalBeliefView method-to-sub-trait mapping

For each of GoalBeliefView's 92 methods, determine which sub-trait it belongs to. This produces the set of sub-traits that GoalBeliefView composes.

### 2. Define GoalBeliefView as a composed supertrait

```rust
pub trait GoalBeliefView:
    EntityBeliefView
    + SpatialBeliefView
    + InventoryBeliefView
    + CombatBeliefView
    + SocialBeliefView
    + EconomicBeliefView
    + PoliticalBeliefView
    + ProfileBeliefView
    + ControlBeliefView
    // + partial TemporalBeliefView / FacilityBeliefView as needed
{
    // Any GoalBeliefView-specific methods not on sub-traits
}
```

Note: If GoalBeliefView includes only a subset of a sub-trait's methods (e.g., 3 of 10 TemporalBeliefView methods), it may still compose the full sub-trait — the unused methods simply have default implementations. This avoids creating sub-sub-traits for partial coverage.

### 3. Update impl_goal_belief_view! macro

Restructure the macro to generate delegation per sub-trait. The generated code is mechanically identical — only the organizational grouping changes.

### 4. Add mock helper traits or default implementations

Provide `unimplemented!()` default implementations on each sub-trait so test mocks can opt out of domains they don't exercise. Alternatively, provide a `MockBeliefViewDefaults` trait or macro that blanket-implements all sub-traits with panicking defaults.

### 5. Simplify existing test mocks

Update the 16 test mock impl blocks to use the new default infrastructure. Each mock should only explicitly implement the sub-traits its test exercises.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — GoalBeliefView as supertrait, macro restructure, default impls)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — updated macro invocation)
- `crates/worldwake-ai/src/planning_state.rs` (modify — updated macro invocation)
- All 16 test mock files (modify — simplified mock implementations)

## Out of Scope

- Further sub-trait extraction (completed in 001-006)
- SnapshotEntity changes (completed in 007)
- Splitting belief_view.rs into multiple files/modules

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. GoalBeliefView remains usable at all existing `&dyn GoalBeliefView` call sites.
2. `impl_goal_belief_view!` macro produces identical runtime behavior as before.
3. Test mocks compile with fewer explicit method implementations than before.
4. No behavioral change.

## Test Plan

### New/Modified Tests

1. None — pure structural refactor. The mock simplification is validated by existing tests compiling and passing.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
