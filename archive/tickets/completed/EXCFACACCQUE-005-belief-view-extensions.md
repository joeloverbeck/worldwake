**Status**: COMPLETED

# EXCFACACCQUE-005 — Belief View Extensions (Queue Position + Grant Query)

**Spec sections**: §9
**Crates**: `worldwake-sim`, `worldwake-ai`

## Summary

Extend the `BeliefView` trait and its omniscient implementation with two new methods for querying local facility queue state. Agents use these to determine their queue position and whether a grant exists.

## Assumptions Reassessed

- `FacilityUseQueue`, `GrantedFacilityUse`, and the queue/grant runtime already exist in the codebase. This ticket is not defining queue state; it is exposing existing authoritative state through the belief abstraction.
- The `BeliefView` trait is implemented by more than `OmniscientBeliefView`. Current implementers include `PlanningState` plus multiple AI and sim test doubles. Any trait extension must update those implementations or the workspace will stop compiling.
- `PlanningSnapshot` and `PlanningState` do not yet snapshot or simulate queue/grant state. That work still belongs to `EXCFACACCQUE-006`. For this ticket, `PlanningState` and test doubles should provide conservative placeholder behavior rather than inventing partial queue simulation.
- Because the architecture requires belief-only planning, exposing queue/grant reads through `BeliefView` is still the correct design. Direct AI reads from `World` would be a regression against the current architecture and foundations.
- The main regression risk is trait-surface churn, not queue logic. Tests therefore need to cover both the new omniscient reads and the compilation/runtime safety of downstream `BeliefView` implementers.

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

### 3. Update non-authoritative `BeliefView` implementers conservatively

Because the trait is already used across `worldwake-ai` and sim test helpers, add minimal implementations where required:
- `PlanningState` returns `None` for both methods until `EXCFACACCQUE-006` teaches snapshots/state about queue and grant data
- test doubles and stubs gain stored fields or conservative `None` implementations as appropriate for their current test scope

Do not add partial queue/grant simulation to planning state in this ticket. That would blur the boundary with `EXCFACACCQUE-006`.

### 4. Locality constraint

Document that these methods return information about facilities only. Future belief implementations (E14+) should gate visibility on colocation or prior knowledge, but the omniscient view returns ground truth.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` — add two trait methods
- `crates/worldwake-sim/src/omniscient_belief_view.rs` — implement both methods
- `crates/worldwake-ai/src/planning_state.rs` — add conservative placeholder implementations
- Existing `BeliefView` stubs/test doubles touched only as needed to satisfy the new trait surface

## Out of Scope

- Planning snapshot or planning state caching (EXCFACACCQUE-006)
- AI planner ops (EXCFACACCQUE-007)
- Candidate generation (EXCFACACCQUE-008)
- Any belief filtering based on E14 perception (future work)
- Modifying any existing belief view methods
- Adding queue/grant fields to `PlanningSnapshot` or hypothetical queue/grant simulation in `PlanningState`

## Acceptance Criteria

### Tests that must pass
- Unit test: `facility_queue_position` returns `Some(0)` for queue head
- Unit test: `facility_queue_position` returns `Some(2)` for third actor in queue
- Unit test: `facility_queue_position` returns `None` for actor not in queue
- Unit test: `facility_queue_position` returns `None` for facility without `FacilityUseQueue`
- Unit test: `facility_grant` returns `Some(grant)` when a grant is active
- Unit test: `facility_grant` returns `None` when no grant is active
- Unit test: `facility_grant` returns `None` for facility without `FacilityUseQueue`
- Unit test: `PlanningState` returns `None` for both methods until queue/grant snapshot support lands in `EXCFACACCQUE-006`
- All affected `BeliefView` test doubles compile without introducing queue/grant behavior outside their current scope
- `cargo test -p worldwake-sim`
- `cargo test -p worldwake-ai`
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- Belief view methods are read-only — they never mutate world state
- No global queue lookup — queries are per-facility
- Methods work correctly with empty queues, single-entry queues, and multi-entry queues
- `GrantedFacilityUse` reference lifetime is tied to the belief view borrow
- `PlanningState` does not pretend to know or simulate queue/grant state before `EXCFACACCQUE-006`

## Outcome

- Completion date: 2026-03-13
- What actually changed:
  - Added `BeliefView::facility_queue_position` and `BeliefView::facility_grant` as conservative default methods returning `None`, so queue/grant reads stay belief-safe without forcing fake implementations into unrelated test doubles.
  - Implemented authoritative queue/grant reads in `OmniscientBeliefView` by reading `FacilityUseQueue` directly from facility components.
  - Added explicit `None` implementations in `PlanningState` to preserve the ticket boundary: snapshot/state queue simulation still belongs to `EXCFACACCQUE-006`.
  - Added coverage for omniscient queue-position and grant queries in sim tests, plus an AI regression test asserting `PlanningState` remains conservative until snapshot support lands.
  - Added minimal `#[allow(clippy::too_many_lines)]` annotations to two pre-existing long regression tests in `crates/worldwake-systems/src/production_actions.rs` so the repository-wide clippy gate passes.
- Deviations from original plan:
  - The trait change did not require manual edits to every test double because `None` is the correct staged default for unsupported queue/grant knowledge.
  - The ticket originally understated the impact radius by omitting `PlanningState` and the broader `BeliefView` implementer set; that was corrected before implementation.
  - No planning snapshot or hypothetical queue/grant overlay work was added here; that remains intentionally deferred to `EXCFACACCQUE-006`.
- Verification results:
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
