# E10PROTRA-010: Travel departure + arrival actions in worldwake-systems

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — travel action support spans `worldwake-sim` action semantics plus `worldwake-systems` registration/handlers
**Deps**: E08 action framework; E10 shared schema already landed (`InTransitOnEdge`, `TravelEdge`, placement/event plumbing exist)

## Problem

Travel must use explicit in-transit occupancy. An entity traveling for ten ticks is physically on the route for ten ticks, not frozen at the source and teleported at arrival. This requires a Travel action that removes the actor from their origin place, assigns `InTransitOnEdge` for the route duration, and on completion removes `InTransitOnEdge` and sets `LocatedIn = destination`.

Carried items remain parented to the carrier through containment, so they travel implicitly because the carrier moves.

## Assumption Reassessment (2026-03-10)

1. `InTransitOnEdge` already exists in `worldwake-core`; this ticket does not need to introduce the component.
2. `TravelEdge` and `Topology` already exist in `crates/worldwake-core/src/topology.rs`, and `travel_time_ticks` is authoritative.
3. Placement is not managed through a standalone `relations.rs` API. The authoritative mutation surface is `World`/`WorldTxn`, especially `set_ground_location`, `effective_place`, `is_in_transit`, and the generated component accessors.
4. The action framework already supports multi-tick start/tick/commit/abort handlers.
5. `EventTag::Travel` already exists. Separate departure/arrival tags are not required for this ticket.
6. Carried items move through containment/possession locality; no separate item-travel path is needed.
7. There is no separate arrival action in the current architecture. Travel should remain one long-running action: departure on start, arrival on commit.
8. The current shared action semantics cannot yet express adjacent-place travel cleanly through affordance generation or duration resolution. That gap must be part of this ticket's scope.
9. The global invariant "every entity is either `LocatedIn` or in transit" is false in the current placement model. This ticket must scope legality to the traveling actor during the action lifecycle, not assert a stronger world-wide rule that the code does not enforce.

## Architecture Check

1. Travel should stay a single long-running action. Splitting departure and arrival into separate public actions would add alias behavior without gaining clarity.
2. A clean implementation needs narrow shared action-layer extensions:
   - adjacent-place target enumeration for affordances
   - travel-duration resolution from the selected edge
   - transit-state filtering so already-traveling actors do not receive travel affordances
3. These shared changes are more beneficial than inventing a travel-only bypass because they preserve one coherent action/affordance architecture for AI, human control, replay, and future travel-adjacent systems.
4. On start: validate the actor has a current place, resolve a single directed edge from origin to destination, clear the actor's current place occupancy, and set `InTransitOnEdge` with departure/arrival ticks.
5. On tick: no-op beyond existing scheduler/body-cost handling. Travel fatigue should continue to use `ActionDef.body_cost_per_tick`, not a one-off handler-side mechanism.
6. On commit: clear `InTransitOnEdge` and place the actor at the destination via the normal placement API.
7. On abort/interruption: clear `InTransitOnEdge` and place the actor back at the origin. This remains the right Phase 2 default because it preserves legality without inventing a stranded-on-edge state before that state has a spec.
8. Duration must come directly from `TravelEdge.travel_time_ticks`.
9. If multiple directed edges share the same `from` and `to`, this ticket should fail loudly rather than inventing alias routing semantics. Parallel-edge selection belongs in a later explicit design.

## What to Change

### 1. New module `crates/worldwake-systems/src/travel_actions.rs`

Define:
- `travel_handler`: `ActionHandler` with start/tick/commit/abort callbacks
  - **start**: validate actor has an origin place, resolve a single origin→destination edge, clear the actor's place occupancy, set `InTransitOnEdge`
  - **tick**: no-op unless `body_cost_per_tick` is non-zero
  - **commit**: remove `InTransitOnEdge`, set `LocatedIn = destination`
  - **abort**: remove `InTransitOnEdge`, set `LocatedIn = origin`
- `ActionDef` for Travel with:
  - actor constraints: alive, controllable, not already in transit
  - targets: destination place entity selected from adjacent places
  - preconditions: destination exists, is a place, and has a directed edge from the actor origin
  - duration: resolved from the chosen edge
  - body cost: explicit `BodyCostPerTick`; use zero unless a concrete travel-cost source is already available
  - visibility: explicit travel visibility consistent with current action events

### 2. Register and export

Add travel action registration in `worldwake-systems` without disturbing existing needs/production registration paths.

### 3. Extend shared action semantics in `worldwake-sim`

Travel cannot be implemented cleanly with the current shared abstractions. Add the minimum shared support needed for lawful travel actions:
- extend `BeliefView` / `OmniscientBeliefView` with adjacency and transit-state queries
- extend `TargetSpec` to enumerate adjacent destination places
- extend `Constraint` and/or `Precondition` so affordance generation and authoritative validation can reject actors already in transit
- extend `DurationExpr` so travel duration comes from the selected edge instead of handler-local duplicated logic
- add a travel payload only if the edge cannot be derived cleanly from actor + target at execution time

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs`
- `crates/worldwake-sim/src/omniscient_belief_view.rs`
- `crates/worldwake-sim/src/action_payload.rs` if a travel payload is required
- `crates/worldwake-sim/src/action_semantics.rs`
- `crates/worldwake-sim/src/action_validation.rs`
- `crates/worldwake-sim/src/affordance_query.rs`
- `crates/worldwake-systems/src/travel_actions.rs`
- `crates/worldwake-systems/src/lib.rs`

No change is expected in `crates/worldwake-core/src/event_tag.rs`; `EventTag::Travel` already exists.

## Out of Scope

- Pick-up / put-down actions (E10PROTRA-011)
- Multi-hop route planning (E13 AI)
- Ambush, escort, or interception logic (E12, future)
- Route danger scoring
- Vehicle or mount systems
- Pathfinding beyond one directed edge chosen by the action affordance
- Parallel-edge route selection semantics when multiple edges share one origin/destination pair

## Acceptance Criteria

### Tests That Must Pass

1. `Travel` creates `InTransitOnEdge` for the full route duration.
2. Arrival removes `InTransitOnEdge` and updates `LocatedIn`.
3. The actor is not at the origin during transit.
4. Carried items remain with the carrier during transit.
5. Travel fails if no directed edge connects origin and destination.
6. Travel fails if the actor is already in transit.
7. Travel duration matches `TravelEdge.travel_time_ticks`.
8. Aborted travel returns the actor to origin with `InTransitOnEdge` removed.
9. Travel events are emitted with causal linkage using existing `ActionStarted` / `ActionCommitted` / `ActionAborted` tags plus `EventTag::Travel`.
10. The action/affordance layer only offers adjacent destinations and does not offer travel while already in transit.
11. No teleportation path moves the actor directly to destination without traversing the edge lifecycle.
12. Existing suites: `cargo test -p worldwake-sim` and `cargo test -p worldwake-systems`.

### Invariants

1. No teleportation: goods and agents move only through physical travel.
2. During an active travel action, the traveling actor is either at a place before departure / after abort-or-commit, or on an edge during transit; never both at once.
3. Carried items move because the carrier/container chain moves; no separate item movement is introduced.
4. Travel is deterministic given the same world state, topology, and action binding.
5. No floating-point arithmetic.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/travel_actions.rs`: happy path (depart→transit→arrive), abort, no-edge failure, already-in-transit failure, carried items, event emission, duration correctness
2. `crates/worldwake-sim/src/affordance_query.rs`, `action_validation.rs`, `action_semantics.rs`, and `omniscient_belief_view.rs`: adjacent-place enumeration, transit gating, and travel-duration resolution

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Spec References

1. `specs/E10-production-transport.md`
2. `specs/IMPLEMENTATION-ORDER.md`
3. `docs/FOUNDATIONS.md`

## Outcome

- Completed: 2026-03-10
- What actually changed:
  - added a generic `travel` action in `worldwake-systems`
  - extended `worldwake-sim` action semantics with adjacent-place targeting, travel-duration resolution, and transit-state gating
  - extended `BeliefView` so affordance generation can reason about adjacency and transit state
  - added `WorldTxn::set_in_transit` / placement support so travel can lawfully clear `LocatedIn` while preserving carried containment trees
  - reused existing `EventTag::Travel`; no new event tags were added
- Deviations from original plan:
  - the ticket originally scoped travel to `worldwake-systems` plus optional event-tag changes
  - implementation required shared action-layer changes in `worldwake-sim` and placement support in `worldwake-core`
  - carried goods do not move "implicitly through containment" from the actor alone in the current architecture; the implementation explicitly moves the actor's directly possessed physical roots with the actor while preserving containment under those roots
- Verification results:
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-systems`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - all passed
