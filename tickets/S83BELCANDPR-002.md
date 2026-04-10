# S83BELCANDPR-002: Add `cognitive_profile()` accessor to GoalBeliefView

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — GoalBeliefView trait extension, RuntimeBeliefView impl
**Deps**: S83BELCANDPR-001

## Problem

The belief-gated candidate filtering (ticket 003) needs to read `CognitiveProfile.speculative_acquisition` through the `GoalBeliefView` trait. Currently `CognitiveProfile` is not accessible via the belief view — it's passed as a direct parameter in `agent_tick`. Other profile types (`ExplorationProfile`, `DisposalProfile`) already have GoalBeliefView accessors with default `None` impls. This ticket adds the same pattern for `CognitiveProfile`.

## Assumption Reassessment (2026-04-10)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:68`. Existing profile accessors: `exploration_profile` (line 198, default `None`), `disposal_profile` (line 194, default `None`), `homeostatic_needs` (line 192). All follow the pattern `fn name(&self, agent: EntityId) -> Option<Type> { None }`.
2. `RuntimeBeliefView` impl in `belief_view.rs` forwards profile accessors through `ProfileBeliefView` trait. The `ProfileBeliefView` trait has its own method set that `GoalBeliefView` blanket-impls forward to.
3. Shared boundary: `GoalBeliefView` is defined in `worldwake-sim`, implemented by `RuntimeBeliefView` (reading from `World` component tables) and by `TestBeliefView` structs in `worldwake-ai` test files. The trait is the abstraction boundary between authoritative state and AI reasoning.

## Architecture Check

1. Follows the exact established pattern for profile accessors on `GoalBeliefView`. No new abstractions, no alternative approaches. The only question was trait accessor vs. parameter threading — trait accessor was chosen for pattern consistency (resolved during reassessment).
2. No backward-compatibility shims. Default `None` impl means all existing `GoalBeliefView` implementors compile without changes.

## Verification Layers

1. `cognitive_profile()` returns correct value for agents with the component -> focused unit test
2. `cognitive_profile()` returns `None` for agents without the component -> focused unit test
3. Single-layer ticket (trait extension with forwarding impl); additional layer mapping not applicable.

## What to Change

### 1. Add method to GoalBeliefView trait

In `crates/worldwake-sim/src/belief_view.rs`, add to the `GoalBeliefView` trait (near the other profile accessors around line 194-200):

```rust
fn cognitive_profile(&self, agent: EntityId) -> Option<CognitiveProfile> {
    let _ = agent;
    None
}
```

Ensure `CognitiveProfile` is imported in the `use` block at the top of the file.

### 2. Add ProfileBeliefView forwarding

In the `ProfileBeliefView` impl block (where `exploration_profile` and `disposal_profile` are forwarded), add the corresponding `cognitive_profile` method that reads from the world's component table.

### 3. Add RuntimeBeliefView blanket impl forwarding

In the `GoalBeliefView` blanket impl for types implementing `ProfileBeliefView`, add the forwarding call for `cognitive_profile`.

### 4. Update TestBeliefView in candidate_generation.rs

In `crates/worldwake-ai/src/candidate_generation.rs`, add `cognitive_profiles: BTreeMap<EntityId, CognitiveProfile>` to the `TestBeliefView` struct and implement the `cognitive_profile()` method to look up from the map.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — if ProfileBeliefView impl lives here)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — TestBeliefView)

## Out of Scope

- Using the accessor in candidate generation logic (ticket 003)
- Adding accessors for any other profile types
- Modifying existing profile accessor patterns

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `cognitive_profile()` returns `Some(profile)` for an agent with CognitiveProfile component
2. New focused test: `cognitive_profile()` returns `None` for an agent without the component
3. Existing suite: `cargo test -p worldwake-sim`
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Default `None` impl means all existing GoalBeliefView implementors (including mock/test impls across the AI crate) compile without changes
2. `RuntimeBeliefView` returns the authoritative CognitiveProfile from the world's component table
3. Workspace builds cleanly: `cargo build --workspace`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` (or test module) — test that RuntimeBeliefView returns CognitiveProfile for agents that have it
2. `crates/worldwake-ai/src/candidate_generation.rs` — TestBeliefView gains `cognitive_profiles` field; existing tests continue passing

### Commands

1. `cargo test -p worldwake-sim belief_view`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
