# S83BELCANDPR-002: Add `cognitive_profile()` accessor to GoalBeliefView

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — GoalBeliefView trait extension, RuntimeBeliefView impl
**Deps**: S83BELCANDPR-001

## Problem

The belief-gated candidate filtering (ticket 003) needs to read `CognitiveProfile.speculative_acquisition` through the `GoalBeliefView` trait. Currently `CognitiveProfile` is not accessible via the belief view — it's passed as a direct parameter in `agent_tick`. Other profile types (`ExplorationProfile`, `DisposalProfile`) already have GoalBeliefView accessors with default `None` impls. This ticket adds the same pattern for `CognitiveProfile`.

## Assumption Reassessment (2026-04-10)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:68`. Existing profile accessors such as `disposal_profile` and `exploration_profile` are default-`None` methods on the goal-facing trait.
2. The real shared boundary for profile access is `ProfileBeliefView` at `crates/worldwake-sim/src/belief_view.rs:429`, because `GoalBeliefView` blanket forwarding reads profile data from that trait rather than from `PerAgentBeliefView` directly.
3. Shared boundary: `GoalBeliefView` is defined in `worldwake-sim`, implemented by runtime readers such as `PerAgentBeliefView` (reading from `World` component tables) and by `TestBeliefView` structs in `worldwake-ai` test files. The trait is the abstraction boundary between authoritative state and AI reasoning.
4. Newly created agents are universally seeded with a default `CognitiveProfile`, so the honest negative runtime proof for this ticket is non-self isolation (`PerAgentBeliefView` does not expose another agent's profile), not an "agent without component" path.

## Architecture Check

1. Follows the exact established pattern for profile accessors on `GoalBeliefView`. No new abstractions, no alternative approaches. The only question was trait accessor vs. parameter threading — trait accessor was chosen for pattern consistency (resolved during reassessment).
2. No backward-compatibility shims. Default `None` on `ProfileBeliefView` keeps existing non-runtime and stub implementors compile-safe unless they actually need cognitive-profile data.

## Verification Layers

1. `cognitive_profile()` returns the authoritative self profile through both `ProfileBeliefView` and `GoalBeliefView` -> focused runtime unit test
2. `cognitive_profile()` does not expose another agent's profile through `PerAgentBeliefView` -> focused runtime unit test
3. Single-layer ticket (trait extension with blanket forwarding); additional layer mapping not applicable.

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

### 2. Add ProfileBeliefView method and GoalBeliefView forwarding

Add `cognitive_profile()` to `ProfileBeliefView` with a default `None` implementation, then forward it through the `GoalBeliefView` blanket impl alongside `exploration_profile()` and `disposal_profile()`.

### 3. Add authoritative runtime implementation

Implement `cognitive_profile()` on `PerAgentBeliefView` by reading the authoritative `CognitiveProfile` component for the acting agent only.

### 4. Update TestBeliefView in candidate_generation.rs

In `crates/worldwake-ai/src/candidate_generation.rs`, add `cognitive_profiles: BTreeMap<EntityId, CognitiveProfile>` to the `TestBeliefView` struct and implement the `cognitive_profile()` method to look up from the map.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — TestBeliefView)

## Out of Scope

- Using the accessor in candidate generation logic (ticket 003)
- Adding accessors for any other profile types
- Modifying existing profile accessor patterns

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `cognitive_profile()` returns `Some(profile)` for the acting agent with CognitiveProfile component
2. New focused test: `cognitive_profile()` returns `None` for a non-self entity in `PerAgentBeliefView`
3. Existing suite: `cargo test -p worldwake-sim`
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Default `None` impl means all existing GoalBeliefView implementors (including mock/test impls across the AI crate) compile without changes
2. `RuntimeBeliefView` returns the authoritative CognitiveProfile from the world's component table
3. Workspace builds cleanly: `cargo build --workspace`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs::cognitive_profile_returns_actor_profile_when_present` — proves the authoritative self profile is readable through both profile surfaces
2. `crates/worldwake-sim/src/per_agent_belief_view.rs::cognitive_profile_returns_none_for_non_self_entity` — proves the runtime view does not expose another agent's profile
3. `crates/worldwake-ai/src/candidate_generation.rs` — `TestBeliefView` gains `cognitive_profiles` support; existing tests continue passing

### Commands

1. `cargo test -p worldwake-sim cognitive_profile_returns_`
2. `cargo build --workspace`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-10.

- Added `cognitive_profile()` to both `ProfileBeliefView` and `GoalBeliefView`, with default `None` behavior on the shared trait surface.
- Wired `GoalBeliefView` blanket forwarding through the new `ProfileBeliefView::cognitive_profile()` method.
- Implemented the authoritative runtime read on `PerAgentBeliefView` and extended the `candidate_generation` test view with a `cognitive_profiles` map so downstream ticket `003` can consume the accessor cleanly.
- Corrected the negative proof surface during implementation: because agents are universally seeded with a default `CognitiveProfile`, the honest runtime isolation check is non-self access returning `None`, not a missing-component agent path.

## Verification Result

- Passed `cargo test -p worldwake-sim cognitive_profile_returns_`
- Passed `cargo build --workspace`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
