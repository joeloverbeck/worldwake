# S59EXPOBLSUB-004: GoalBeliefView trait extension

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — GoalBeliefView trait in worldwake-sim
**Deps**: S59EXPOBLSUB-002

## Problem

Candidate generation reads agent data through the `GoalBeliefView` trait. For search goal emission (ticket 011) to read `ExpectationStore` and `LastSeenMemory`, new trait methods must be added. Without these, the AI cannot query expectation state during goal formation.

## Assumption Reassessment (2026-04-06)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:32`. Default methods return `None`/empty — new methods follow same pattern.
2. `PerAgentBeliefView` is the primary impl at `crates/worldwake-ai/src/planning_state.rs`. It reads from `World` component accessors.
3. Test mocks in `candidate_generation.rs` and `exhaustion.rs` implement the trait with default stubs. New methods auto-default to `None`.
4. `ExpectationStore` and `LastSeenMemory` types are in `worldwake-core`, which `worldwake-sim` depends on — no crate boundary issue.

## Architecture Check

1. Adding default-returning methods to GoalBeliefView is non-breaking — all existing impls continue to compile. Only `PerAgentBeliefView` needs an override to read from World.
2. No backward compatibility shims.

## Verification Layers

1. Trait methods return component data → focused unit test on PerAgentBeliefView impl
2. Single-layer ticket (trait extension) — additional layer mapping not applicable.

## What to Change

### 1. Add trait methods

In `crates/worldwake-sim/src/belief_view.rs`, add to `GoalBeliefView`:

```rust
fn expectation_store(&self, agent: EntityId) -> Option<ExpectationStore> {
    let _ = agent;
    None
}
fn last_seen_memory(&self, agent: EntityId) -> Option<LastSeenMemory> {
    let _ = agent;
    None
}
```

Add `ExpectationStore` and `LastSeenMemory` to the import block.

### 2. Implement in PerAgentBeliefView

In `crates/worldwake-ai/src/planning_state.rs`, implement both methods on `PerAgentBeliefView` by delegating to `self.world.component_expectation_store(agent)` and `self.world.component_last_seen_memory(agent)`, cloning the result.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait methods + imports)
- `crates/worldwake-ai/src/planning_state.rs` (modify — implement methods on PerAgentBeliefView)

## Out of Scope

- Using the methods in candidate generation — ticket 011
- Test mock implementations — auto-default to None, no changes needed

## Acceptance Criteria

### Tests That Must Pass

1. `PerAgentBeliefView::expectation_store(agent)` returns the agent's store when set
2. `PerAgentBeliefView::last_seen_memory(agent)` returns the agent's memory when set
3. Default stub returns `None` for agents without the component
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. All existing GoalBeliefView impls continue to compile (default methods)
2. No test mock changes required

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_state.rs` (inline test) — verify PerAgentBeliefView returns expectation/last-seen data

### Commands

1. `cargo test -p worldwake-ai planning_state`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
