# S102FROAWAEXP-003: GoalBeliefView accessor for exhaustion count

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — ProfileBeliefView trait, GoalBeliefView blanket impl, PerAgentBeliefView backing
**Deps**: archive/tickets/S102FROAWAEXP-002.md

## Problem

The exploration gate in candidate_generation.rs reads agent state through `GoalBeliefView`. The new `AcquisitionExhaustionTracker` component (ticket 002) has no accessor in this trait chain, so the AI crate cannot read exhaustion counts during candidate generation.

## Assumption Reassessment (2026-04-14)

1. `ProfileBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:446-482` contains `exploration_profile()` at line 454 with default `let _ = agent; None`. The new method follows this exact pattern.
2. Blanket `GoalBeliefView` impl at `belief_view.rs:916-1550+` forwards `exploration_profile` at lines 1255-1260 via `ProfileBeliefView::exploration_profile(self, agent)`. New method needs identical forwarding.
3. `GoalBeliefView` trait at `belief_view.rs:69-853` contains method signatures with defaults. New method needs a default returning 0.
4. `PerAgentBeliefView` at `crates/worldwake-sim/src/per_agent_belief_view.rs` implements `RuntimeBeliefView` and already treats profile-style reads as self-authoritative. The new method must follow that same boundary: read the actor's `AcquisitionExhaustionTracker` through `World::get_component_acquisition_exhaustion_tracker()` and return `0` for non-self queries.
5. No `impl_goal_belief_view!` macro exists — forwarding is via blanket impl pattern.

## Architecture Check

1. Following the established `ProfileBeliefView` → blanket `GoalBeliefView` → `PerAgentBeliefView` chain is the canonical pattern. Every profile accessor uses this path. No new traits or special-casing required.
2. No backward-compatibility shims. Default returning `0` preserves existing test doubles, and the runtime implementation stays honest by exposing authoritative counts only for `self`, matching the existing profile-read privacy boundary.

## Verification Layers

1. Stub/default trait path returns `0` for all agents/needs → focused unit test on the blanket `GoalBeliefView` consumer surface
2. Runtime self-read returns actual tracker state → focused `PerAgentBeliefView` test
3. Runtime non-self read stays hidden and live self default reads as `0` → focused `PerAgentBeliefView` test
4. Single-crate trait/accessor plumbing remains coherent through crate and workspace verification

## What to Change

### 1. Add method to ProfileBeliefView

In `crates/worldwake-sim/src/belief_view.rs`, add to `ProfileBeliefView` trait (alongside `exploration_profile`):

```rust
fn acquisition_exhaustion_count(&self, agent: EntityId, need: HomeostaticNeedId) -> u8 {
    let _ = (agent, need);
    0
}
```

### 2. Add method to GoalBeliefView trait

Add the same signature with default returning 0 to the `GoalBeliefView` trait definition.

### 3. Add forwarding in blanket GoalBeliefView impl

In the `impl<T> GoalBeliefView for T where T: ...` block, add:

```rust
fn acquisition_exhaustion_count(&self, agent: EntityId, need: HomeostaticNeedId) -> u8 {
    ProfileBeliefView::acquisition_exhaustion_count(self, agent, need)
}
```

### 4. Implement in PerAgentBeliefView

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, implement `acquisition_exhaustion_count` on the `ProfileBeliefView` impl for `PerAgentBeliefView` to read `AcquisitionExhaustionTracker` from the world via `get_component_acquisition_exhaustion_tracker()`, while preserving the existing self-authoritative profile boundary by returning `0` for non-self queries.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — ProfileBeliefView trait, GoalBeliefView trait, blanket impl)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — runtime backing)

## Out of Scope

- Planner failure tracking that writes to the tracker (ticket 004)
- Exploration gate that reads the accessor (ticket 004)
- Test belief views in candidate_generation.rs — they inherit the default (returns 0), overridden only in golden test tickets

## Acceptance Criteria

### Tests That Must Pass

1. Default `ProfileBeliefView::acquisition_exhaustion_count` returns 0
2. `PerAgentBeliefView` returns the live self tracker count through both `ProfileBeliefView` and `GoalBeliefView`
3. `PerAgentBeliefView` returns `0` for live self default state and for non-self queries
4. Workspace builds cleanly: `cargo build --workspace`
5. Existing suite: `cargo test --workspace`

### Invariants

1. All existing test belief views continue to compile without modification (default returns 0)
2. GoalBeliefView blanket impl forwards to ProfileBeliefView
3. Runtime tracker counts remain self-authoritative; non-self reads do not expose authoritative state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` — unit test confirming the default/forwarded accessor returns `0`
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` — focused test confirming runtime self reads return the stored tracker count
3. `crates/worldwake-sim/src/per_agent_belief_view.rs` — focused test confirming runtime self default and non-self reads return `0`

### Commands

1. `cargo test -p worldwake-sim --lib belief_view::tests::goal_belief_view_acquisition_exhaustion_count_defaults_to_zero -- --exact`
2. `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::acquisition_exhaustion_count_returns_actor_tracker_count_when_present -- --exact`
3. `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::acquisition_exhaustion_count_returns_zero_for_non_self_and_default_live_agent -- --exact`
4. `cargo test -p worldwake-sim`
5. `cargo build --workspace`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-04-14

Added `acquisition_exhaustion_count` to both `ProfileBeliefView` and `GoalBeliefView`, forwarded it through the blanket `GoalBeliefView` impl, and backed the runtime read in `PerAgentBeliefView` from the authoritative `AcquisitionExhaustionTracker` component. The landed runtime contract matches the existing profile-read boundary: self queries read authoritative state, while non-self queries return `0`.

Deviations from original plan:
- The ticket originally described a `query_acquisition_exhaustion_tracker()` path and a more generic "component may be absent" read shape. The live codebase uses `World::get_component_acquisition_exhaustion_tracker()` and universally seeds the component for agents, so the honest remaining distinction was self-authoritative access vs. non-self suppression.
- Focused proof widened from one default-only test to three tests so the ticket proves the default trait path, explicit runtime self reads, and the non-self/default-zero boundary separately.

## Verification Result

- Passed: `cargo test -p worldwake-sim --lib belief_view::tests::goal_belief_view_acquisition_exhaustion_count_defaults_to_zero -- --exact`
- Passed: `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::acquisition_exhaustion_count_returns_actor_tracker_count_when_present -- --exact`
- Passed: `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::acquisition_exhaustion_count_returns_zero_for_non_self_and_default_live_agent -- --exact`
- Passed: `cargo test -p worldwake-sim`
- Passed: `cargo build --workspace`
- Passed: `cargo test --workspace`
- Passed: `cargo clippy --workspace --all-targets -- -D warnings`
