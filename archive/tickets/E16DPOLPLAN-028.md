# E16DPOLPLAN-028: Add courage to the belief observation pipeline

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core (belief structs, observation snapshot), worldwake-sim (PerAgentBeliefView)
**Deps**: E16DPOLPLAN-001 (courage field on UtilityProfile and SnapshotEntity)

## Problem

`PerAgentBeliefView::courage()` only returns courage for `self` (the querying agent). For all other agents, it returns `None`. This means the planning snapshot never has courage data for targets, causing `apply_threaten_for_office` to always default targets to `pm(1000)` (max courage, always resist). The planner can never select Threaten as a viable operation.

The belief pipeline already captures observable properties (place, inventory, wounds, alive status) through `ObservedEntitySnapshot` → `BelievedEntityState` → `PerAgentBeliefView`. Courage must follow the same path so that agents who have observed another agent know that agent's courage — enabling the Threaten planner to compare `attack_skill` vs believed courage.

## Assumption Reassessment (2026-03-18)

1. `ObservedEntitySnapshot` has fields: place, inventory, workstation_tag, resource_source, alive, wounds — no courage — confirmed
2. `BelievedEntityState` mirrors the same fields — no courage — confirmed
3. `PerAgentBeliefView::courage()` is self-only (line 511: `agent == self.agent`) — confirmed
4. `PerAgentBeliefView::wounds()` follows the correct pattern: self → read world, other → read beliefs — confirmed
5. `UtilityProfile.courage` exists on all agents created via `seed_agent` (set via `set_component_utility_profile`) — confirmed
6. `PlanningState::courage()` reads from `SnapshotEntity.courage` which is populated from `view.courage(entity)` — confirmed; fixing the view fixes the snapshot

## Architecture Check

1. **Follows established belief pipeline pattern**: The exact same approach used for wounds, place, inventory, alive — no new abstractions needed. Courage becomes an observable property captured during perception, stored in beliefs, readable from beliefs for known entities.
2. **Principle 13 compliance**: Courage knowledge travels through the perception pipeline. An agent only knows another's courage if they have observed them. Beliefs can become stale if courage changes between observations.
3. **Principle 12 compliance**: No omniscience. The agent reads believed courage, not authoritative world courage. The planning snapshot correctly reads from beliefs.
4. **No backwards-compatibility shims**: Direct extension of existing structs and methods.

## What to Change

### 1. Add courage to `ObservedEntitySnapshot` (worldwake-core)

In `crates/worldwake-core/src/belief.rs`:

- Add `pub courage: Option<Permille>` to `ObservedEntitySnapshot`
- In `build_observed_entity_snapshot()`, capture courage: `courage: world.get_component_utility_profile(entity).map(|p| p.courage)`

### 2. Add courage to `BelievedEntityState` (worldwake-core)

In `crates/worldwake-core/src/belief.rs`:

- Add `pub last_known_courage: Option<Permille>` to `BelievedEntityState`
- In `to_believed_entity_state()`, copy: `last_known_courage: self.courage`

### 3. Update `PerAgentBeliefView::courage()` (worldwake-sim)

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, change `courage()` to follow the `wounds()` pattern:

```rust
fn courage(&self, agent: EntityId) -> Option<Permille> {
    if agent == self.agent {
        return self.world
            .get_component_utility_profile(agent)
            .map(|p| p.courage);
    }
    self.believed_entity(agent)
        .and_then(|state| state.last_known_courage)
}
```

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)

## Out of Scope

- Courage perception fidelity or distortion (future E14 work)
- Courage inference from observed behavior (e.g., inferring courage from combat performance)
- Changes to `SnapshotEntity` or `PlanningState` (they already read from the belief view correctly)

## Acceptance Criteria

### Tests That Must Pass

1. `courage_returns_profile_value_for_self_and_none_for_others` — update: should now return believed courage for observed agents
2. New test: `courage_returns_believed_value_for_observed_other_agent` — observer with beliefs about target reads target's believed courage
3. New test: `courage_returns_none_for_unknown_agent` — observer without beliefs about target gets None
4. New test: `build_observed_entity_snapshot_captures_courage` — snapshot includes courage from UtilityProfile
5. Existing suite: `cargo test -p worldwake-core` and `cargo test -p worldwake-sim`

### Invariants

1. Belief-only planning (Principle 12): courage for other agents comes exclusively from beliefs, never from authoritative world state
2. Locality (Principle 13): courage is acquired through observation and stored in the belief store
3. No regression: self-courage queries continue to work identically

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — test that `build_observed_entity_snapshot` captures courage
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` — update existing courage test, add believed-other and unknown-other tests

### Commands

1. `cargo test -p worldwake-core belief`
2. `cargo test -p worldwake-sim per_agent_belief_view`
3. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-18
- **What changed**: Added `courage: Option<Permille>` to `ObservedEntitySnapshot`, `last_known_courage: Option<Permille>` to `BelievedEntityState`, updated `build_observed_entity_snapshot()` to capture courage from `UtilityProfile`, updated `to_believed_entity_state()` to propagate it, and changed `PerAgentBeliefView::courage()` to return believed courage for observed agents (following the `wounds()` pattern). Updated 20+ construction sites across core/sim/systems/ai crates.
- **Deviations**: None — implemented exactly as specified.
- **Verification**: `cargo build --workspace` passes, `cargo test --workspace` 1954 tests pass (0 failures), `cargo clippy --workspace` clean. New tests: `build_observed_entity_snapshot_captures_courage`, `courage_returns_profile_value_for_self_and_believed_for_observed`, `courage_returns_none_for_observed_agent_without_courage_belief`, `courage_returns_none_for_unknown_agent`.
