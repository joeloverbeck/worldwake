# EXCFACACCQUE-005 — Belief View Extensions (Queue Position + Grant Query)

**Spec sections**: §9
**Crates**: `worldwake-sim`

## Summary

Extend the `BeliefView` trait and its omniscient implementation with two new methods for querying local facility queue state. Agents use these to determine their queue position and whether a grant exists.

## Deliverables

### 1. Add methods to `BeliefView` trait

In `crates/worldwake-sim/src/belief_view.rs`, add:

```rust
/// Returns the actor's 0-indexed position in the facility's queue,
/// or None if the actor is not queued at this facility.
fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32>;

/// Returns the current active grant at this facility, if any.
fn facility_grant(&self, facility: EntityId) -> Option<&GrantedFacilityUse>;
```

### 2. Implement on `OmniscientBeliefView`

In `crates/worldwake-sim/src/omniscient_belief_view.rs`:
- `facility_queue_position`: read `FacilityUseQueue` component from the facility entity, call `position_of(actor)`
- `facility_grant`: read `FacilityUseQueue` component, return `granted.as_ref()`

### 3. Locality constraint

Document that these methods return information about facilities only. Future belief implementations (E14+) should gate visibility on colocation or prior knowledge, but the omniscient view returns ground truth.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` — add two trait methods
- `crates/worldwake-sim/src/omniscient_belief_view.rs` — implement both methods

## Out of Scope

- Planning snapshot or planning state caching (EXCFACACCQUE-006)
- AI planner ops (EXCFACACCQUE-007)
- Candidate generation (EXCFACACCQUE-008)
- Any belief filtering based on E14 perception (future work)
- Modifying any existing belief view methods

## Acceptance Criteria

### Tests that must pass
- Unit test: `facility_queue_position` returns `Some(0)` for queue head
- Unit test: `facility_queue_position` returns `Some(2)` for third actor in queue
- Unit test: `facility_queue_position` returns `None` for actor not in queue
- Unit test: `facility_queue_position` returns `None` for facility without `FacilityUseQueue`
- Unit test: `facility_grant` returns `Some(grant)` when a grant is active
- Unit test: `facility_grant` returns `None` when no grant is active
- Unit test: `facility_grant` returns `None` for facility without `FacilityUseQueue`
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- Belief view methods are read-only — they never mutate world state
- No global queue lookup — queries are per-facility
- Methods work correctly with empty queues, single-entry queues, and multi-entry queues
- `GrantedFacilityUse` reference lifetime is tied to the belief view borrow
