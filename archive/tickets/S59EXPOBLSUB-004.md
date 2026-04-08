# S59EXPOBLSUB-004: GoalBeliefView trait extension

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — GoalBeliefView trait in worldwake-sim
**Deps**: S59EXPOBLSUB-002

## Problem

Candidate generation reads agent data through the `GoalBeliefView` trait. For search goal emission (ticket 011) to read `ExpectationStore` and `LastSeenMemory`, new trait methods must be added. Without these, the AI cannot query expectation state during goal formation.

## Assumption Reassessment (2026-04-06)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:32`. Default methods return `None`/empty — new methods follow same pattern.
2. `PerAgentBeliefView` is the primary live consumer boundary, but it lives in `crates/worldwake-sim/src/per_agent_belief_view.rs`, not `worldwake-ai`. It reaches `GoalBeliefView` through the `impl_goal_belief_view!` forwarding macro in `crates/worldwake-sim/src/belief_view.rs`.
3. Test mocks in `candidate_generation.rs` and `exhaustion.rs` implement the trait with default stubs. New methods auto-default to `None`.
4. `ExpectationStore` and `LastSeenMemory` types are in `worldwake-core`, which `worldwake-sim` depends on — no crate boundary issue.
5. Ticket says only `GoalBeliefView` and `worldwake-ai/src/planning_state.rs` need edits. Live code differs: the honest implementation boundary is `GoalBeliefView` defaults, `RuntimeBeliefView` forwarding support, the `impl_goal_belief_view!` macro, and `PerAgentBeliefView`'s `RuntimeBeliefView` impl inside `worldwake-sim`. Correction applied: keep the change entirely in `worldwake-sim`, with sim-side tests. Safe because the live symbol ownership is unambiguous.

## Architecture Check

1. Adding default-returning methods to `GoalBeliefView` remains non-breaking for existing direct impls. Because `PerAgentBeliefView` gets `GoalBeliefView` through the forwarding macro, the runtime trait and macro forwarding path must also be widened inside `worldwake-sim`.
2. No backward compatibility shims.

## Verification Layers

1. Default `GoalBeliefView` stubs return `None` → focused trait-default unit test
2. `PerAgentBeliefView` returns component data through the runtime + forwarding path → focused sim-side unit test
3. Single-layer ticket (trait extension) — additional layer mapping not applicable.

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

In `crates/worldwake-sim/src/belief_view.rs`, add matching default methods to `RuntimeBeliefView` and forward them through `impl_goal_belief_view!`.

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, implement both runtime methods on `PerAgentBeliefView` by delegating to `self.world.get_component_expectation_store(agent)` and `self.world.get_component_last_seen_memory(agent)`, cloning the result only for `self.agent`.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait/runtime methods + forwarding macro + tests)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — implement methods on PerAgentBeliefView + tests)

## Out of Scope

- Using the methods in candidate generation — ticket 011
- Test mock implementations — auto-default to None, no changes needed

## Acceptance Criteria

### Tests That Must Pass

1. `GoalBeliefView` default stub returns `None` for agents without the component
2. `PerAgentBeliefView::expectation_store(agent)` returns the agent's store when set
3. `PerAgentBeliefView::last_seen_memory(agent)` returns the agent's memory when set
4. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. All existing GoalBeliefView impls continue to compile (default methods)
2. No test mock changes required

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` — verify new default trait methods return `None`
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` — verify PerAgentBeliefView returns expectation/last-seen data for self

### Commands

1. `cargo test -p worldwake-sim belief_view`
2. `cargo test -p worldwake-sim per_agent_belief_view`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

Completed on 2026-04-06.

- Added `expectation_store()` and `last_seen_memory()` default methods to both `GoalBeliefView` and `RuntimeBeliefView` in `crates/worldwake-sim/src/belief_view.rs`.
- Extended the `impl_goal_belief_view!` forwarding macro so runtime-backed views expose the new methods through the canonical goal-read surface.
- Implemented self-authoritative reads for both components in `crates/worldwake-sim/src/per_agent_belief_view.rs`.
- Added focused sim-side tests proving default stub behavior and `PerAgentBeliefView` self-only visibility.
- Bounded deviation from the original ticket wording: the live implementation boundary was entirely inside `worldwake-sim`, not `worldwake-ai/src/planning_state.rs`.

## Verification Result

- Passed `cargo test -p worldwake-sim goal_belief_view_expectation_defaults_return_none`
- Passed `cargo test -p worldwake-sim self_expectation_and_last_seen_queries_are_authoritative_only_for_self`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
