# S06COMOPPVAL-002: Extend GoalBeliefView with commodity_valuation_profile

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-sim` (GoalBeliefView trait extension + PerAgentBeliefView implementation)
**Deps**: S06COMOPPVAL-001

## Problem

The shared commodity-opportunity layer needs to read an agent's `CommodityValuationProfile` through the belief view to determine reasoning depth, place horizon, and decay. Without this trait method, `commodity_opportunity_score` (ticket 003) cannot access per-agent valuation bounds in a belief-facing way.

## Assumption Reassessment (2026-04-02)

1. `GoalBeliefView` at `crates/worldwake-sim/src/belief_view.rs:30` is the narrow AI-facing trait. It currently has no `commodity_valuation_profile` method.
2. `PerAgentBeliefView` at `crates/worldwake-sim/src/per_agent_belief_view.rs` implements `GoalBeliefView` by reading components from `World` or `AgentBeliefStore`. Adding a new method follows the existing pattern: read component from world, return `Option<T>`.
3. `RuntimeBeliefView` at `belief_view.rs:319` is the broader trait, and the repo currently uses `impl_goal_belief_view!` in `belief_view.rs` to forward many `GoalBeliefView` methods from `RuntimeBeliefView` implementors. In practice, new self-authoritative profile reads must be mirrored there with a default `None` to preserve that forwarding architecture, even though the canonical consumer surface remains `GoalBeliefView`.
4. `GoalBeliefView` and `RuntimeBeliefView` both already use default methods that return `None` or empty for optional components. The new method should follow that default pattern on both traits, with `PerAgentBeliefView` supplying the real self-authoritative override.

## Architecture Check

1. The canonical valuation consumer boundary remains `GoalBeliefView`, which keeps S06 belief-facing and narrow. A mirrored default on `RuntimeBeliefView` is an implementation consequence of the existing forwarding macro, not a change in who should depend on the method.
2. No backward-compatibility shims. The new trait methods both default to `None`, existing implementors stay source-compatible, and only `PerAgentBeliefView` opts into the real self-authoritative read.

## Verification Layers

1. `GoalBeliefView` exposes `commodity_valuation_profile` on the narrow valuation surface -> focused trait/read test
2. `PerAgentBeliefView` returns the profile when component exists -> focused unit test
3. `PerAgentBeliefView` returns `None` when component absent -> focused unit test
4. `impl_goal_belief_view!` continues forwarding profile reads from `RuntimeBeliefView` implementors -> compiler plus focused trait test
5. Single-layer ticket (trait extension, no runtime behavior change).

## What to Change

### 1. Add method to `GoalBeliefView` trait

In `crates/worldwake-sim/src/belief_view.rs`, add to `GoalBeliefView`:

```rust
fn commodity_valuation_profile(&self, agent: EntityId) -> Option<CommodityValuationProfile> {
    let _ = agent;
    None
}
```

Import `CommodityValuationProfile` from `worldwake_core`.

### 2. Mirror on `RuntimeBeliefView` and forwarding macro

Because `PerAgentBeliefView`, `PlanningState`, and several test/runtime stubs rely on `impl_goal_belief_view!` to derive `GoalBeliefView` from `RuntimeBeliefView`, add the same defaulted method to `RuntimeBeliefView` and extend the forwarding macro to call `RuntimeBeliefView::commodity_valuation_profile(self, agent)`.

This is an implementation-detail mirror, not a widening of the intended S06 consumer boundary.

### 3. Implement on `PerAgentBeliefView`

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, implement the method by reading the component from the world:

```rust
fn commodity_valuation_profile(&self, agent: EntityId) -> Option<CommodityValuationProfile> {
    self.world.get_component_commodity_valuation_profile(agent).copied()
}
```

The exact accessor name depends on the schema macro expansion from ticket 001.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add method to `GoalBeliefView`)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — implement method and focused tests)

## Out of Scope

- Commodity opportunity module (ticket 003)
- Recipe propagation (ticket 004)
- Trade/AI integration (tickets 005, 006)

## Acceptance Criteria

### Tests That Must Pass

1. `PerAgentBeliefView::commodity_valuation_profile` returns `Some(profile)` when component is set on agent
2. `PerAgentBeliefView::commodity_valuation_profile` returns `None` when component is absent
3. `GoalBeliefView` callers reach the same value through the forwarding surface for `PerAgentBeliefView`
4. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `GoalBeliefView` remains the canonical narrow AI-facing valuation surface.
2. The mirrored `RuntimeBeliefView` method exists only to preserve the existing forwarding architecture used by `impl_goal_belief_view!`; new valuation consumers should still depend on `GoalBeliefView`.
3. Default implementation returns `None` — existing implementors are unaffected.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` or `belief_view.rs` test module — focused tests for the new method and `GoalBeliefView` forwarding

### Commands

1. `cargo test -p worldwake-sim -- belief_view` — targeted tests
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite

## Outcome

- Completed: 2026-04-02
- What changed:
  - Extended [`GoalBeliefView`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) with `commodity_valuation_profile`.
  - Mirrored the method on `RuntimeBeliefView` and forwarded it through `impl_goal_belief_view!` in [`belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) so the existing runtime-backed architecture stayed intact.
  - Implemented the real self-authoritative read in [`PerAgentBeliefView`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) using `get_component_commodity_valuation_profile`.
  - Added focused present/absent and forwarding-path tests in [`per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs).
- Deviations from original plan:
  - The live codebase did not have `PerAgentBeliefView` implementing `GoalBeliefView` directly. The correct boundary remained `GoalBeliefView`, but the implementation had to also extend `RuntimeBeliefView` and the forwarding macro to preserve the repo's existing trait-pair architecture.
- Verification results:
  - `cargo test -p worldwake-sim commodity_valuation_profile -- --nocapture`
  - `cargo test -p worldwake-sim belief_view -- --nocapture`
  - `cargo test -p worldwake-sim`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
