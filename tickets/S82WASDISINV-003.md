# S82WASDISINV-003: Add DisposalProfile belief-view accessor

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new trait method on ProfileBeliefView
**Deps**: S82WASDISINV-001

## Problem

The AI crate needs to read `DisposalProfile` (specifically `capacity_strain_threshold`) during candidate generation and goal satisfaction checks. The belief-view trait hierarchy requires a new accessor method following the existing profile accessor pattern.

## Assumption Reassessment (2026-04-10)

1. `ProfileBeliefView` trait exists in `crates/worldwake-sim/src/belief_view.rs`. Existing profile accessors include `perception_profile`, `cognitive_profile`, `combat_profile`, etc.
2. `RuntimeBeliefView` at line 814 is a supertrait unifying 11 belief view categories. `GoalBeliefView` has a blanket impl at lines 875-913 forwarding through `ProfileBeliefView`.
3. Forwarding is done via direct `impl` blocks (not a macro). `GoalBeliefView` blanket impl includes `ProfileBeliefView` as a bound.
4. `per_agent_belief_view.rs` contains the `PerAgentBeliefView` struct that implements `ProfileBeliefView` — this is where the actual world-state reading happens.

## Architecture Check

1. Follows the exact existing pattern for profile accessors (`perception_profile`, `combat_profile`, etc.). No new abstractions needed.
2. No backward-compatibility shims. New accessor only.

## Verification Layers

1. `disposal_profile()` returns correct profile for agents with DisposalProfile set -> focused unit test
2. `disposal_profile()` returns `None` for agents without DisposalProfile -> focused unit test
3. Single-layer ticket (belief-view accessor) — no cross-system verification needed

## What to Change

### 1. ProfileBeliefView trait method

In `crates/worldwake-sim/src/belief_view.rs`, add to `ProfileBeliefView` trait:

```rust
fn disposal_profile(&self, entity: EntityId) -> Option<DisposalProfile>;
```

### 2. PerAgentBeliefView implementation

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, implement the new method by reading from the world's component store (same pattern as other profile accessors).

### 3. Test mock implementations

Update any test mock implementations of `ProfileBeliefView` to include the new method.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- Any test files with mock `ProfileBeliefView` implementations (modify)

## Out of Scope

- Using the accessor in candidate generation (ticket 007)
- Using the accessor in goal satisfaction (ticket 005)
- Changes to `GoalBeliefView` blanket impl (already forwards via `ProfileBeliefView` bound)

## Acceptance Criteria

### Tests That Must Pass

1. `disposal_profile(agent)` returns `Some(DisposalProfile { capacity_strain_threshold: Permille(800) })` for an agent with default DisposalProfile
2. `disposal_profile(agent)` returns `None` for an agent without DisposalProfile set
3. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Belief-view trait hierarchy remains coherent — all implementors compile
2. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (test module) — verify accessor reads correct profile
2. `crates/worldwake-sim/src/belief_view.rs` (test module) — verify mock returns expected value

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
