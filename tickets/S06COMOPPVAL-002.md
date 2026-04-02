# S06COMOPPVAL-002: Extend GoalBeliefView with commodity_valuation_profile

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-sim` (GoalBeliefView trait extension + PerAgentBeliefView implementation)
**Deps**: S06COMOPPVAL-001

## Problem

The shared commodity-opportunity layer needs to read an agent's `CommodityValuationProfile` through the belief view to determine reasoning depth, place horizon, and decay. Without this trait method, `commodity_opportunity_score` (ticket 003) cannot access per-agent valuation bounds in a belief-facing way.

## Assumption Reassessment (2026-04-02)

1. `GoalBeliefView` at `crates/worldwake-sim/src/belief_view.rs:30` is the narrow AI-facing trait. It currently has no `commodity_valuation_profile` method.
2. `PerAgentBeliefView` at `crates/worldwake-sim/src/per_agent_belief_view.rs` implements `GoalBeliefView` by reading components from `World` or `AgentBeliefStore`. Adding a new method follows the existing pattern: read component from world, return `Option<T>`.
3. `RuntimeBeliefView` at `belief_view.rs:319` is the broader trait. Per the spec, the new method goes on `GoalBeliefView` (narrow surface), not `RuntimeBeliefView`.
4. `GoalBeliefView` has default methods that return `None` or empty for optional components (pattern: `fn foo(&self, agent: EntityId) -> Option<T> { let _ = agent; None }`). The new method should follow this default pattern.

## Architecture Check

1. Placing `commodity_valuation_profile` on `GoalBeliefView` (not `RuntimeBeliefView`) keeps the valuation boundary narrow — only goal formation and valuation scoring can access it, not the broader affordance/search machinery.
2. No backward-compatibility shims. New trait method with default returning `None` — existing implementors compile without changes until they opt in.

## Verification Layers

1. `PerAgentBeliefView` returns the profile when component exists -> focused unit test
2. `PerAgentBeliefView` returns `None` when component absent -> focused unit test
3. Single-layer ticket (trait extension, no runtime behavior change).

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

### 2. Implement on `PerAgentBeliefView`

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, implement the method by reading the component from the world:

```rust
fn commodity_valuation_profile(&self, agent: EntityId) -> Option<CommodityValuationProfile> {
    self.world.get_component_commodity_valuation_profile(agent).copied()
}
```

The exact accessor name depends on the schema macro expansion from ticket 001.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add method to `GoalBeliefView`)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — implement method)

## Out of Scope

- Commodity opportunity module (ticket 003)
- Recipe propagation (ticket 004)
- Trade/AI integration (tickets 005, 006)

## Acceptance Criteria

### Tests That Must Pass

1. `PerAgentBeliefView::commodity_valuation_profile` returns `Some(profile)` when component is set on agent
2. `PerAgentBeliefView::commodity_valuation_profile` returns `None` when component is absent
3. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `GoalBeliefView` remains the narrow AI-facing surface — no broad runtime helpers added.
2. Default implementation returns `None` — existing implementors are unaffected.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` or `belief_view.rs` test module — focused tests for the new method

### Commands

1. `cargo test -p worldwake-sim -- belief_view` — targeted tests
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite
