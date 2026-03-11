# E13DECARC-011: Candidate-scoped PlanningSnapshot and PlanningState overlay

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None - AI-layer types and tests
**Deps**: `specs/E13-decision-architecture.md`, existing `worldwake-ai` goal/planner primitives, `worldwake-sim::BeliefView`

## Problem

E13 already has candidate generation, ranking, planning budgets, and planner-op metadata, but it still lacks the hypothetical belief-state layer that bounded search needs. The planner needs a candidate-scoped immutable snapshot plus a compact transient overlay so future search can evaluate legal affordances without cloning authoritative world state or mutating runtime beliefs.

## Assumption Reassessment (2026-03-11)

1. `BeliefView` currently has 38 methods in [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs), not a smaller pre-E13 surface.
2. `get_affordances(view: &dyn BeliefView, actor, registry)` already exists in [`crates/worldwake-sim/src/affordance_query.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/affordance_query.rs) and returns `Vec<Affordance>`.
3. `Affordance` does contain `def_id`, `actor`, `bound_targets`, and `payload_override`, plus `explanation`.
4. `worldwake-ai` does not currently contain `planning_snapshot.rs` or `planning_state.rs`; this ticket must create them instead of modifying stubs.
5. `worldwake-ai` already contains E13 planner primitives in [`crates/worldwake-ai/src/planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs), [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), and [`crates/worldwake-ai/src/budget.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/budget.rs).
6. The previous dependency references to `E13DECARC-005` and `E13DECARC-010` do not match active ticket files under `tickets/`; this ticket must depend on the E13 spec and the code that already landed, not on missing ticket documents.
7. `HomeostaticNeeds`, `DriveThresholds`, `Wound`, `InTransitOnEdge`, `MerchandiseProfile`, `DemandObservation`, and `ResourceSource` exist in `worldwake-core`, but authoritative wounds are stored as `WoundList`; planner state must avoid inventing a parallel authoritative wound model.
8. `cargo test -p worldwake-ai` is currently green before this work; this is new infrastructure, not a repair of a failing test.

## Architecture Check

1. A candidate-scoped snapshot is better than a world clone because it preserves Principle 7 locality and keeps search costs proportional to evidence and travel horizon rather than total world size.
2. A transient overlay is better than mutating copied entity records per node because it keeps branch state explicit, cheap to fork, and impossible to confuse with authoritative world state.
3. The clean boundary is:
   - `PlanningSnapshot`: immutable, candidate-local, cloned once per planning attempt
   - `PlanningState<'s>`: borrowed overlay, cheap to branch, implements `BeliefView`
4. The snapshot should be entity-centric, not a pile of special-case caches. Place membership, possession, resource-source data, seller visibility, and travel edges should derive from a single candidate-local record model so later E13 tickets can extend it without duplicating storage.
5. The overlay should only model hypothetical deltas the planner actually needs:
   - place changes
   - possession/container changes
   - resource quantity changes
   - reservation shadowing
   - target removal / death
   - conservative need and pain updates
6. This is more robust than the current architecture because the current architecture has no hypothetical-state layer at all. The alternative is letting search read live runtime beliefs directly or cloning broad view state ad hoc inside search, and both paths would be harder to extend and easier to get wrong.

## Scope Correction

This ticket delivers the hypothetical-state substrate only.

In scope:
- define `PlanningSnapshot`
- define `PlanningState<'s>`
- implement `BeliefView` for `PlanningState<'s>`
- build snapshots from `&dyn BeliefView`
- apply a small, conservative set of planner-visible hypothetical deltas needed by the next search ticket
- add focused tests proving snapshot boundaries and overlay behavior

Out of scope:
- bounded search itself
- final plan selection
- full goal satisfaction / progress-barrier semantics
- runtime plan revalidation
- blocked-intent writing
- full agent tick integration

## What To Change

### 1. Add `PlanningSnapshot` in `crates/worldwake-ai/src/planning_snapshot.rs`

Create an immutable candidate-scoped snapshot built from `&dyn BeliefView`.

Required properties:
- actor id
- actor-local body state needed by planning
- candidate-local entity records for the actor, evidence entities, reachable places, and entities at included places
- candidate-local place membership
- candidate-local travel edges within the requested horizon
- demand memory and merchandise profile cloned once into the snapshot, never per node

The exact field layout may differ from the old sketch if the implementation remains entity-centric and deterministic.

### 2. Add `PlanningState<'s>` in `crates/worldwake-ai/src/planning_state.rs`

Create a borrowed overlay over `PlanningSnapshot`.

Required delta support:
- entity place overrides
- direct possessor / direct container overrides
- resource quantity overrides
- reservation shadowing
- local target removal / death markers
- conservative body-state overrides needed for planner simulation

`PlanningState<'s>` must be transient only and must not be registered as a component.

### 3. Implement `BeliefView` for `PlanningState<'s>`

`PlanningState<'s>` must satisfy the full trait surface, even when some methods are snapshot fallbacks.

Critical behaviors:
- no override -> exact snapshot answer
- `effective_place()` must respect overlay movement first
- `direct_possessions()` / `direct_possessor()` / `direct_container()` must reflect overlay moves
- `resource_source()` and `resource_sources_at()` must reflect quantity overrides
- `is_dead()` / `is_alive()` / `entities_at()` must hide locally removed targets
- `reservation_conflicts()` must account for planner shadow reservations so future search does not double-book hypothetical steps

### 4. Add snapshot construction helper

Provide a constructor along the lines of:

```rust
pub fn build_planning_snapshot(
    view: &dyn BeliefView,
    actor: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    evidence_places: &BTreeSet<EntityId>,
    travel_horizon: u8,
) -> PlanningSnapshot
```

The implementation may expose `PlanningSnapshot::build(...)` instead if that keeps the API tighter.

### 5. Add conservative overlay application helpers

Provide a small set of state-advance helpers for the next ticket. They do not need to model the full simulation; they only need to update the hypothetical fields that search and goal checks will read.

Minimum required support for this ticket:
- travel updates actor place
- consume updates actor needs conservatively
- cargo transfer updates possession/container state
- resource-use updates source availability

If the implementation uses explicit helper methods instead of one large `apply_op()` function, that is preferred.

## Files To Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (new)
- `crates/worldwake-ai/src/planning_state.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (export wiring)

## Acceptance Criteria

### Tests That Must Pass

1. `PlanningState<'_>` satisfies `BeliefView`.
2. With no overrides, `PlanningState<'_>` returns the same answers as its `PlanningSnapshot`.
3. Snapshot construction includes the actor, all evidence entities, all evidence places, and places reachable within the requested travel horizon.
4. Snapshot construction does not pull in unrelated remote places outside the travel horizon.
5. Movement overlays update `effective_place()` and place membership consistently.
6. Possession/container overlays update `direct_possessor()`, `direct_container()`, and `direct_possessions()` consistently.
7. Resource quantity overlays change `resource_source()` and `resource_sources_at()` consistently.
8. Local target-removal overlays make removed entities disappear from `is_alive()`, `is_dead()`, and `entities_at()`.
9. Reservation shadowing causes `reservation_conflicts()` to report conflicts for hypothetically reserved entities.
10. `get_affordances(&planning_state, actor, registry)` works against hypothetical state and reflects overlay changes.
11. Heavy vectors such as actor wounds and demand memory are stored once in the snapshot and are not re-cloned for each overlay branch.
12. Existing suite: `cargo test -p worldwake-ai`
13. Existing suite: `cargo test --workspace`
14. Existing suite: `cargo clippy --workspace`

### Invariants

1. `PlanningSnapshot` is immutable after construction.
2. `PlanningState<'_>` never becomes authoritative world state.
3. Snapshot and overlay stay deterministic: `BTreeMap` / `BTreeSet` only.
4. No backward-compatibility aliasing or duplicate planner-state model.
5. No full world clone.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs`
   Rationale: verifies candidate-scoped extraction and horizon/evidence boundaries.
2. `crates/worldwake-ai/src/planning_state.rs`
   Rationale: verifies overlay semantics, `BeliefView` behavior, and affordance-query compatibility.
3. `crates/worldwake-ai/src/lib.rs`
   Rationale: verifies the new planning types are exported from the crate boundary used by later E13 tickets.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

Completed: 2026-03-11
Outcome amended: 2026-03-11

What actually changed:
- Added owned `PlanningSnapshot` and borrowed `PlanningState<'snapshot>` in `worldwake-ai`.
- Extended `worldwake-sim::BeliefView` with the raw planner inputs needed to keep hypothetical planning belief-local: reservation ranges plus metabolism, trade-disposition, and combat profiles.
- Added `estimate_duration_from_beliefs(...)` in `worldwake-sim` so duration estimation can resolve from `BeliefView` data instead of live-world access.
- Kept the snapshot candidate-scoped and deterministic, but removed the earlier borrowed-view fallback by snapshotting reservation ranges and planner-relevant profile data directly into entity records.
- Wired the new types through `worldwake-ai` crate exports.
- Added focused tests for snapshot horizon boundaries, overlay semantics, affordance-query compatibility, and heavy-vector sharing across overlay branches.

Deviations from original plan:
- The original ticket assumed existing stub files and ticket dependencies that did not exist; those were corrected first.
- The final design is more entity-centric than the original hand-written place-cache sketch to avoid duplicating the same world facts in multiple planner-specific maps.
- The overlay exposes explicit helper methods (`move_actor_to`, `move_lot_to_holder`, `consume_commodity`, `use_resource`, `reserve`, `mark_removed`) instead of one large speculative `apply_op()` entry point.
- The implementation went beyond the first completed cut by making the planning snapshot fully self-contained instead of retaining a borrowed fallback for reservation and duration queries.

Verification:
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace`
