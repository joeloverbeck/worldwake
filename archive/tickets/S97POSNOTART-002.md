# S97POSNOTART-002: `GoalBeliefView` accessor for posting profile

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new profile accessor carried from `PerAgentBeliefView` through planner snapshot/state into `GoalBeliefView`
**Deps**: archive/tickets/S97POSNOTART-001.md

## Problem

The AI crate's candidate generation accesses agent profile components through `GoalBeliefView` (via `GenerationContext.view`). Without an accessor for `ArtifactPostingProfile`, the candidate generation code cannot read TTL values to set `expires_at` on posted artifacts.

## Assumption Reassessment (2026-04-12)

1. `GoalBeliefView` and `ProfileBeliefView` at `crates/worldwake-sim/src/belief_view.rs` already expose agent-profile reads through a blanket forwarding impl. Existing pattern: `drive_thresholds(agent) -> Option<DriveThresholds>` and `disposal_profile(agent) -> Option<DisposalProfile>`. The new accessor should follow that pattern.
2. `PerAgentBeliefView` is the authoritative runtime reader for self-profile components, and `PlanningSnapshot` / `PlanningState` in `worldwake-ai` explicitly enumerate carried profile fields. `ArtifactPostingProfile` is not yet present there, so snapshot carriage is part of this ticket's honest scope.
3. No existing accessor for `ArtifactPostingProfile` exists in `belief_view.rs`, `per_agent_belief_view.rs`, `planning_snapshot.rs`, or `planning_state.rs`.

## Architecture Check

1. Following the established profile-accessor pattern is the cleanest fit for the existing abstraction boundary: authoritative self-profile reads live on `PerAgentBeliefView`, planner-visible reads come from snapshot-carried data, and `GoalBeliefView` reaches them through `ProfileBeliefView`.
2. No backward-compatibility shims — add the accessor once on `ProfileBeliefView`, expose it through the blanket `GoalBeliefView` impl, and extend the explicit planner snapshot payload instead of creating side channels.

## Verification Layers

1. Runtime authoritative profile read returns profile data for the owning agent → focused sim test on `PerAgentBeliefView`
2. Planner snapshot/state round-trip preserves the profile → focused AI test
3. Accessor returns `None` for entities without the profile → focused default-path assertion

## What to Change

### 1. Add trait method to `ProfileBeliefView` and expose it through `GoalBeliefView`

In `crates/worldwake-sim/src/belief_view.rs`, add to `ProfileBeliefView`:

```rust
fn artifact_posting_profile(&self, agent: EntityId) -> Option<ArtifactPostingProfile> {
    None
}
```

Then add the corresponding forwarding method on the blanket `impl<T> GoalBeliefView for T`.

### 2. Implement in `PerAgentBeliefView`

Add the backing implementation in `crates/worldwake-sim/src/per_agent_belief_view.rs` that reads `ArtifactPostingProfile` from authoritative component storage for `self.agent`, following the same pattern as `drive_thresholds` and `disposal_profile`.

### 3. Carry the profile through planning snapshot/state

Add the field to `SnapshotProfiles` in `crates/worldwake-ai/src/planning_snapshot.rs`, populate it in `build_snapshot_entity`, and expose it from `PlanningState`'s `ProfileBeliefView` impl in `crates/worldwake-ai/src/planning_state.rs`.

### 4. Add import

Import `ArtifactPostingProfile` from `worldwake-core` where needed in sim/ai files and focused tests.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — `ProfileBeliefView` accessor + blanket `GoalBeliefView` forwarding)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — authoritative profile read + focused test if colocated)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — snapshot profile carriage + focused test)
- `crates/worldwake-ai/src/planning_state.rs` (modify — planner accessor + focused test support)

## Out of Scope

- Candidate generation changes (ticket 003)
- CLI scenario support (ticket 004)
- Golden tests (ticket 005)
- Candidate generation changes that consume the accessor (ticket 003)

## Acceptance Criteria

### Tests That Must Pass

1. `PerAgentBeliefView` / `GoalBeliefView` surface returns `Some(profile)` for an agent with the component
2. Planner snapshot/state preserves the profile value for the actor
3. Accessor returns `None` when the profile is absent
4. Existing focused suites pass in `worldwake-sim` and `worldwake-ai`

### Invariants

1. `GoalBeliefView` accessor follows the same pattern as existing profile accessors — default `None` on `ProfileBeliefView`, forwarded through the blanket impl
2. No new crate dependencies introduced

## Test Plan

### New/Modified Tests

1. `worldwake-sim` focused test proving authoritative self-profile access returns `ArtifactPostingProfile`
2. `worldwake-ai` focused test proving snapshot/state round-trip preserves `ArtifactPostingProfile`

### Commands

1. `cargo test -p worldwake-sim artifact_posting_profile`
2. `cargo test -p worldwake-ai artifact_posting_profile`
3. `cargo test -p worldwake-sim`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed as a cross-crate profile-carriage slice. `ArtifactPostingProfile` is now readable through `GoalBeliefView` / `ProfileBeliefView`, backed authoritatively by `PerAgentBeliefView`, and preserved through `PlanningSnapshot` into `PlanningState` so later candidate-generation work can consume it from planner-visible state.

## Deviations

1. The original draft treated this as a sim-only accessor ticket and referenced a nonexistent `impl_goal_belief_view!` macro. Reassessment corrected the live boundary: the blanket `GoalBeliefView` impl forwards `ProfileBeliefView`, and planner-visible access required explicit snapshot/state carriage in `worldwake-ai`.

## Verification Result

1. Passed `cargo test -p worldwake-sim artifact_posting_profile`
2. Passed `cargo test -p worldwake-ai artifact_posting_profile`
3. Passed `cargo test -p worldwake-sim`
4. Passed `cargo test -p worldwake-ai`
5. Passed `cargo clippy --workspace --all-targets -- -D warnings`
