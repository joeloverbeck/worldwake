# S80EXPDRI-002: Belief view accessor and scenario wiring

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new GoalBeliefView method, AgentDef field, spawn_agent update
**Deps**: S80EXPDRI-001

## Problem

Candidate generation (ticket 004) needs to read `ExplorationProfile` through the `GoalBeliefView` trait during the agent tick. Without the accessor, the AI crate cannot access the profile. Additionally, scenario authors cannot configure exploration behavior per-agent without `AgentDef` wiring and `spawn_agent` support.

## Assumption Reassessment (2026-04-10)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:67` currently has ~40 methods with default implementations returning `None`/empty. Pattern: `fn method(&self, agent: EntityId) -> Option<Type>` with `let _ = agent; None` default body.
2. Live forwarding does not use an `impl_goal_belief_view!` macro. `GoalBeliefView` is implemented via blanket forwarding over sub-traits in `crates/worldwake-sim/src/belief_view.rs`, so the owned boundary is `ProfileBeliefView` plus the `GoalBeliefView for T` forwarding block.
3. Shared abstraction boundary: `GoalBeliefView` trait is the belief-mediated read surface for the AI crate (P26 — systems interact through state). Adding an accessor follows the established pattern for all other profile reads.
4. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:66` has ~30 `Option<ProfileType>` fields. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:310` uses `unwrap_or_default()` for universal profiles.

## Architecture Check

1. Follows the exact established pattern: GoalBeliefView accessor → blanket forwarding through the narrow belief sub-traits → RuntimeBeliefView reads from ECS store. AgentDef uses `Option<ExplorationProfile>` with `unwrap_or_default()` in spawn_agent for universal profiles. No novel architecture.
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

### 2. Forward through the live blanket impl

Add `exploration_profile` to `ProfileBeliefView`, then forward it through the `GoalBeliefView for T` blanket implementation so `PerAgentBeliefView` and other runtime views satisfy the trait.

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

- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait method + blanket forwarding + RuntimeBeliefView-facing read surface)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — add ExplorationProfile field to AgentDef)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — set component in spawn_agent)

## Outcome

Completed on 2026-04-10.

- Added `exploration_profile()` to the AI-facing belief surface by extending `ProfileBeliefView`, forwarding it through the `GoalBeliefView` blanket impl, and implementing the runtime read in `PerAgentBeliefView` via `get_component_exploration_profile`.
- Added `AgentDef.exploration_profile: Option<ExplorationProfile>` and wired `spawn_agent()` to seed the authoritative component with `unwrap_or_default()` so the universal-profile contract matches live world bootstrap behavior.
- Added focused coverage for runtime belief reads, scenario RON deserialization, default scenario seeding, and explicit scenario overrides.
- Updated CLI test scenario builders and direct `AgentDef` literals that needed the new optional field during all-target constructor fallout.

## Deviations

- The ticket referenced an `impl_goal_belief_view!` macro, but the live codebase uses blanket forwarding from `GoalBeliefView` into narrower sub-traits. The implementation followed the live boundary instead of introducing a new macro path.
- The spec example field names for `ExplorationProfile` did not match the already-landed core component shape. Tests and scenario wiring now use the live fields from `crates/worldwake-core/src/exploration.rs`.

## Verification Result

- Passed `cargo test -p worldwake-sim`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo build --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`

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
