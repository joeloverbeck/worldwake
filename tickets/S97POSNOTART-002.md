# S97POSNOTART-002: `GoalBeliefView` accessor for posting profile

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new trait method on `GoalBeliefView`, impl on `RuntimeBeliefView`
**Deps**: archive/tickets/S97POSNOTART-001.md

## Problem

The AI crate's candidate generation accesses agent profile components through `GoalBeliefView` (via `GenerationContext.view`). Without an accessor for `ArtifactPostingProfile`, the candidate generation code cannot read TTL values to set `expires_at` on posted artifacts.

## Assumption Reassessment (2026-04-12)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:69` provides accessor methods for profile components. Existing pattern: `drive_thresholds(agent) -> Option<DriveThresholds>` (line 194), `cognitive_profile(agent) -> Option<CognitiveProfile>` (line 211), `utility_profile(agent) -> Option<UtilityProfile>` (line 324). The new accessor follows this exact pattern.
2. `RuntimeBeliefView` implements `GoalBeliefView` with backing reads from `PlanningSnapshot`. The `impl_goal_belief_view!` macro forwards trait methods. Both need the new method.
3. No existing accessor for `ArtifactPostingProfile` exists (confirmed: zero grep matches in `belief_view.rs`).

## Architecture Check

1. Following the established accessor pattern (trait default returning `None`, `RuntimeBeliefView` impl reading from snapshot, macro forwarding) is cleaner than any alternative — it maintains the existing abstraction boundary between AI planning and authoritative state.
2. No backward-compatibility shims — new trait method with default impl, so existing implementations compile without changes.

## Verification Layers

1. Accessor returns profile data for agents that have it → focused unit test on `RuntimeBeliefView`
2. Accessor returns `None` for entities without the profile → default impl test
3. Single-layer ticket (belief-view infrastructure only) — no cross-system verification needed.

## What to Change

### 1. Add trait method to `GoalBeliefView`

In `crates/worldwake-sim/src/belief_view.rs`, add to the `GoalBeliefView` trait:

```rust
fn artifact_posting_profile(&self, agent: EntityId) -> Option<ArtifactPostingProfile> {
    None
}
```

### 2. Implement in `RuntimeBeliefView`

Add the backing implementation that reads `ArtifactPostingProfile` from the planning snapshot's component data, following the same pattern as `drive_thresholds`, `cognitive_profile`, etc.

### 3. Forward through `impl_goal_belief_view!` macro

Add the new method to the macro's forwarding list so blanket implementations delegate correctly.

### 4. Add import

Import `ArtifactPostingProfile` from `worldwake-core` in `belief_view.rs`.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait method, RuntimeBeliefView impl, macro forwarding)
- `crates/worldwake-sim/Cargo.toml` (modify — only if `ArtifactPostingProfile` re-export requires it; likely no change since worldwake-sim already depends on worldwake-core)

## Out of Scope

- Candidate generation changes (ticket 003)
- CLI scenario support (ticket 004)
- Golden tests (ticket 005)
- Snapshot population of the profile data (handled by existing snapshot infrastructure if the component is registered)

## Acceptance Criteria

### Tests That Must Pass

1. `RuntimeBeliefView::artifact_posting_profile(agent)` returns `Some(profile)` for an agent with the component
2. Default trait impl returns `None`
3. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. `GoalBeliefView` accessor follows the same pattern as existing profile accessors — default `None`, override in `RuntimeBeliefView`
2. No new crate dependencies introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` (test module) — test accessor returns profile for agent with component
2. None — documentation-only aspects; verification is command-based.

### Commands

1. `cargo test -p worldwake-sim -- belief_view`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace --all-targets -- -D warnings`
