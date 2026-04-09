# S80EXPDRI-002: Belief view accessor and scenario wiring

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new GoalBeliefView method, AgentDef field, spawn_agent update
**Deps**: S80EXPDRI-001

## Problem

Candidate generation (ticket 004) needs to read `ExplorationProfile` through the `GoalBeliefView` trait during the agent tick. Without the accessor, the AI crate cannot access the profile. Additionally, scenario authors cannot configure exploration behavior per-agent without `AgentDef` wiring and `spawn_agent` support.

## Assumption Reassessment (2026-04-10)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:67` currently has ~40 methods with default implementations returning `None`/empty. Pattern: `fn method(&self, agent: EntityId) -> Option<Type>` with `let _ = agent; None` default body.
2. `impl_goal_belief_view!` macro forwards trait methods to `RuntimeBeliefView`. Located in `crates/worldwake-sim/src/belief_view.rs`. New methods need forwarding entries.
3. Shared abstraction boundary: `GoalBeliefView` trait is the belief-mediated read surface for the AI crate (P26 — systems interact through state). Adding an accessor follows the established pattern for all other profile reads.
4. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:66` has ~30 `Option<ProfileType>` fields. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:310` uses `unwrap_or_default()` for universal profiles.

## Architecture Check

1. Follows the exact established pattern: GoalBeliefView accessor → macro forwarding → RuntimeBeliefView reads from ECS store. AgentDef uses `Option<ExplorationProfile>` with `unwrap_or_default()` in spawn_agent for universal profiles. No novel architecture.
2. No backward-compatibility shims. New accessor with default `None` return — existing impls remain valid.

## Verification Layers

1. GoalBeliefView accessor returns profile for agents that have one → focused unit test on RuntimeBeliefView
2. spawn_agent applies ExplorationProfile with defaults when absent in RON → scenario loading test
3. spawn_agent applies custom ExplorationProfile when present in RON → scenario loading test
4. Single-layer ticket (wiring/plumbing); additional layer mapping not applicable.

## What to Change

### 1. Add GoalBeliefView accessor

In `crates/worldwake-sim/src/belief_view.rs`, add to `GoalBeliefView` trait:

```rust
fn exploration_profile(&self, agent: EntityId) -> Option<ExplorationProfile> {
    let _ = agent;
    None
}
```

### 2. Forward in impl_goal_belief_view! macro

Add `exploration_profile` to the macro forwarding list so `PerAgentBeliefView` and other runtime views satisfy the trait.

### 3. Implement in RuntimeBeliefView

Read `ExplorationProfile` from the ECS store via `get_component_exploration_profile`.

### 4. Add to AgentDef

In `crates/worldwake-cli/src/scenario/types.rs`, add field:

```rust
pub exploration_profile: Option<ExplorationProfile>,
```

### 5. Wire in spawn_agent

In `crates/worldwake-cli/src/scenario/mod.rs` `spawn_agent()`, add:

```rust
txn.set_component_exploration_profile(
    agent_id,
    agent_def.exploration_profile.clone().unwrap_or_default(),
);
```

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait method + macro forwarding + RuntimeBeliefView impl)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — add ExplorationProfile field to AgentDef)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — set component in spawn_agent)

## Out of Scope

- Goal dispatch and planner integration (ticket 003)
- Candidate generation logic that reads the profile (ticket 004)
- Golden E2E tests (ticket 005)
- ExplorationProfileDef wrapper type (not needed — no EntityId references in the profile)

## Acceptance Criteria

### Tests That Must Pass

1. GoalBeliefView::exploration_profile returns Some for agents with the component
2. AgentDef without exploration_profile field loads and applies defaults
3. AgentDef with custom exploration_profile applies specified values
4. Existing suite: `cargo test -p worldwake-sim`
5. Existing suite: `cargo test -p worldwake-cli`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. GoalBeliefView default returns None — existing test mocks unaffected
2. Universal profile contract: spawn_agent always sets ExplorationProfile (unwrap_or_default)
3. P26: AI crate reads profile only through GoalBeliefView, never directly from ECS store

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` (test module) — RuntimeBeliefView returns ExplorationProfile for agent
2. `crates/worldwake-cli/src/scenario/` (test module) — scenario round-trip with and without exploration_profile in RON

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-cli`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo build --workspace`
