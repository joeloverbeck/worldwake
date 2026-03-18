# E16DPOLPLAN-001: Add `courage()` to `RuntimeBeliefView` trait + implementations

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — belief_view trait, omniscient + per-agent impls
**Deps**: None

## Problem

The GOAP planner needs to read target courage during Threaten evaluation, but `RuntimeBeliefView` has no `courage()` method.

## Assumption Reassessment (2026-03-18)

1. `RuntimeBeliefView` trait is in `crates/worldwake-sim/src/belief_view.rs` — confirmed
2. `OmniscientBeliefView` is in `crates/worldwake-sim/src/omniscient_belief_view.rs` — confirmed
3. `PerAgentBeliefView` is in `crates/worldwake-sim/src/per_agent_belief_view.rs` — confirmed
4. `UtilityProfile.courage` field exists in `crates/worldwake-core/src/utility_profile.rs` — confirmed
5. `combat_profile()` method exists on `RuntimeBeliefView` as the pattern to follow — confirmed

## Architecture Check

1. Default method with `None` return follows existing trait pattern (see `combat_profile()`, `trade_disposition_profile()`)
2. Both impls delegate to `world.get_component_utility_profile(agent).map(|p| p.courage)` — no new abstractions

## What to Change

### 1. `RuntimeBeliefView` trait — add default method

Add after `combat_profile()` (~line 186):
```rust
fn courage(&self, agent: EntityId) -> Option<Permille> {
    let _ = agent;
    None
}
```

### 2. `OmniscientBeliefView` — implement

```rust
fn courage(&self, agent: EntityId) -> Option<Permille> {
    self.world.get_component_utility_profile(agent).map(|p| p.courage)
}
```

### 3. `PerAgentBeliefView` — implement

Same delegation pattern as OmniscientBeliefView.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)

## Out of Scope

- Observation gating for courage (post-E14 per-agent belief boundaries)
- Changes to `UtilityProfile` struct
- Any changes to worldwake-ai crate

## Acceptance Criteria

### Tests That Must Pass

1. `courage()` returns `Some(value)` for an agent that has a `UtilityProfile` with `courage` set
2. `courage()` returns `None` for entities without `UtilityProfile`
3. Both `OmniscientBeliefView` and `PerAgentBeliefView` return identical results
4. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. `RuntimeBeliefView` default returns `None` — no silent behavior change for implementors that don't override
2. No new component types introduced

## Test Plan

### New/Modified Tests

1. Unit test in `belief_view.rs` or `omniscient_belief_view.rs` — verify `courage()` reads `UtilityProfile.courage`

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace`
