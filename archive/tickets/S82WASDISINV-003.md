# S82WASDISINV-003: Add DisposalProfile belief-view accessor

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new disposal-profile accessor on belief-view traits
**Deps**: S82WASDISINV-001

## Problem

The AI crate consumes `GoalBeliefView`, not `ProfileBeliefView` directly, when reading profile-backed planning inputs. This ticket adds a `DisposalProfile` accessor following the existing optional-profile pattern so candidate generation and goal satisfaction work can read `capacity_strain_threshold` through the live belief-view hierarchy.

## Assumption Reassessment (2026-04-10)

1. `ProfileBeliefView` trait exists in `crates/worldwake-sim/src/belief_view.rs`. Existing profile accessors include `perception_profile`, `cognitive_profile`, `combat_profile`, etc.
2. `GoalBeliefView` itself exposes profile-backed accessors such as `exploration_profile`, `preference_profile`, and `utility_profile`, with blanket forwarding in `belief_view.rs:1207-1374`. `candidate_generation.rs` reads profiles through `ctx.view: &dyn GoalBeliefView`, so adding only a `ProfileBeliefView` method would not make the disposal profile reachable at the AI call site.
3. `ProfileBeliefView` optional profile accessors (`exploration_profile`, `preference_profile`, `utility_profile`) default to `None` rather than forcing every test stub to implement them. `DisposalProfile` should follow that optional-accessor pattern at the trait surface even though live agents currently receive `DisposalProfile::default()` at creation time.
4. `PerAgentBeliefView` in `per_agent_belief_view.rs` contains the authoritative component read for profile accessors. Because agent creation now seeds `DisposalProfile::default()` in `world.rs`, the honest negative case for this ticket is a non-self entity or entity without a disposal component, not the primary live actor path.

## Architecture Check

1. Follows the exact existing optional-profile accessor pattern already used by `exploration_profile`, `preference_profile`, and `utility_profile`: add the method to both trait surfaces, forward it through the blanket `GoalBeliefView` impl, and keep the authoritative read in `PerAgentBeliefView`.
2. No backward-compatibility shims. New accessor only.

## Verification Layers

1. `GoalBeliefView::disposal_profile()` forwards through the blanket impl -> focused unit test
2. `PerAgentBeliefView::disposal_profile()` returns the actor's default/live profile -> focused unit test
3. `PerAgentBeliefView::disposal_profile()` returns `None` for a non-self or profile-missing entity -> focused unit test
4. Single-layer ticket (belief-view accessor) — no cross-system verification needed

## What to Change

### 1. Belief-view trait methods

In `crates/worldwake-sim/src/belief_view.rs`, add `disposal_profile()` to `ProfileBeliefView` and `GoalBeliefView`, following the existing optional-profile/default-`None` pattern. Forward the `GoalBeliefView` method through the blanket impl the same way `exploration_profile`, `preference_profile`, and `utility_profile` already forward through `ProfileBeliefView`.

### 2. PerAgentBeliefView implementation

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, implement the new method by reading from the world's component store (same pattern as other profile accessors).

### 3. Focused proof updates

Update focused `worldwake-sim` tests so they prove both `ProfileBeliefView` and `GoalBeliefView` see the same disposal profile for the live actor path, and that the accessor still returns `None` for the negative case.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)

## Out of Scope

- Using the accessor in candidate generation (ticket 007)
- Using the accessor in goal satisfaction (ticket 005)
- Any disposal-planning semantics beyond exposing the belief-view accessor

## Acceptance Criteria

### Tests That Must Pass

1. `GoalBeliefView::disposal_profile(agent)` returns `Some(DisposalProfile { capacity_strain_threshold: Permille(800) })` for a live agent with the default seeded profile
2. `PerAgentBeliefView::disposal_profile(other)` returns `None` when the queried entity is not the viewing agent or lacks the component
3. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Belief-view trait hierarchy remains coherent — `GoalBeliefView` and `ProfileBeliefView` expose the same disposal-profile contract for forwarding implementations
2. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (test module) — verify both `ProfileBeliefView` and `GoalBeliefView` return the live actor's disposal profile
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` (test module) — verify negative case returns `None`

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-04-10
- Changed `crates/worldwake-sim/src/belief_view.rs` to add `disposal_profile()` to both `GoalBeliefView` and `ProfileBeliefView`, with blanket forwarding from the caller-facing goal surface through the profile surface.
- Changed `crates/worldwake-sim/src/per_agent_belief_view.rs` to read `DisposalProfile` from authoritative component state for the viewing agent and added focused tests covering both the live actor path and the non-self negative case.
- Deviations from original plan: reassessment showed the live AI call sites consume `GoalBeliefView`, not `ProfileBeliefView` directly, so the honest owned boundary included the caller-facing goal trait and its blanket forwarding rather than only a new `ProfileBeliefView` method. Reassessment also showed agents now spawn with `DisposalProfile::default()`, so the negative test case was corrected from "agent without DisposalProfile" to the lawful non-self/profile-missing path.
- Verification results:
  - `cargo test -p worldwake-sim disposal_profile`
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace --all-targets -- -D warnings`
