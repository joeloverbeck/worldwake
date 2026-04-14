# S102FROAWAEXP-003: GoalBeliefView accessor for exhaustion count

**Status**: PENDING
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
4. `PerAgentBeliefView` at `crates/worldwake-sim/src/per_agent_belief_view.rs` implements `RuntimeBeliefView` (empty impl block at line 1130) — inherits all trait methods via blanket impl. The `ProfileBeliefView` impl for `PerAgentBeliefView` needs the new method backed by ECS read.
5. No `impl_goal_belief_view!` macro exists — forwarding is via blanket impl pattern.

## Architecture Check

1. Following the established `ProfileBeliefView` → blanket `GoalBeliefView` → `PerAgentBeliefView` chain is the canonical pattern. Every profile accessor uses this path. No new traits or special-casing required.
2. No backward-compatibility shims. Default returning 0 means all existing test belief views work without modification.

## Verification Layers

1. Default returns 0 for all agents/needs → focused unit test with test belief view
2. Runtime read returns actual tracker state → integration-level test via PerAgentBeliefView
3. Single-layer ticket (trait plumbing) — compilation proves the chain works

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

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, implement `acquisition_exhaustion_count` on the `ProfileBeliefView` impl for `PerAgentBeliefView` to read `AcquisitionExhaustionTracker` from the world via `query_acquisition_exhaustion_tracker()`.

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
2. Workspace builds cleanly: `cargo build --workspace`
3. Existing suite: `cargo test --workspace`

### Invariants

1. All existing test belief views continue to compile without modification (default returns 0)
2. GoalBeliefView blanket impl forwards to ProfileBeliefView

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` — unit test confirming default return value of 0

### Commands

1. `cargo test -p worldwake-sim -- belief_view`
2. `cargo build --workspace && cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
