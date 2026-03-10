# E10PROTRA-010: Travel departure + arrival actions in worldwake-systems

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new ActionDefs + ActionHandlers for travel
**Deps**: E10PROTRA-003 (InTransitOnEdge component must exist)

## Problem

Travel must use explicit in-transit occupancy. An entity traveling for ten ticks is physically on the route for ten ticks, not frozen at the source and teleported at arrival. This requires a Travel action that: removes the actor from their origin place, assigns `InTransitOnEdge` for the route duration, and upon completion removes `InTransitOnEdge` and sets `LocatedIn = destination`.

Carried items remain parented to the carrier through containment — they travel implicitly because the carrier moves.

## Assumption Reassessment (2026-03-10)

1. `InTransitOnEdge` component will exist on Agent entities after E10PROTRA-003 — confirmed.
2. `TravelEdge` and `Topology` exist in `worldwake-core/src/topology.rs` — travel edges have `travel_time_ticks`.
3. The relation system has `LocatedIn` placement in `relations.rs` — `set_located_in()` and similar APIs.
4. The action framework supports multi-tick actions with start/tick/commit/abort.
5. Carried items use the ownership/containment relation — items owned by an agent are implicitly at the agent's location. When the agent is in transit, the items are also in transit through containment.
6. There is no separate "arrival" action — travel is a single action with departure at start and arrival at commit.

## Architecture Check

1. Travel is a single long-running action, not two separate actions. Start = departure, commit = arrival.
2. On start: remove actor from origin place (`LocatedIn`), set `InTransitOnEdge` with computed arrival tick.
3. On tick: no-op (or apply body cost for travel fatigue if `body_cost_per_tick` is set).
4. On commit: remove `InTransitOnEdge`, set `LocatedIn = destination`.
5. On abort: the spec doesn't fully specify interrupted travel. Reasonable default: actor returns to origin (they turn around). Alternative: actor stays on the edge in a stranded state. Recommend: abort returns to origin for Phase 2 simplicity, with event logging.
6. Duration comes from `TravelEdge.travel_time_ticks`.
7. This satisfies the route-presence requirement for later ambush, escort, witness, and interception logic.

## What to Change

### 1. New module `crates/worldwake-systems/src/travel_actions.rs`

Define:
- `travel_handler`: ActionHandler with start/tick/commit/abort callbacks
  - **start**: Validate actor is at origin place, validate travel edge exists and connects origin→destination, remove actor from origin `LocatedIn`, set `InTransitOnEdge`
  - **tick**: Apply body cost if applicable
  - **commit**: Remove `InTransitOnEdge`, set `LocatedIn = destination`, emit arrival event
  - **abort**: Remove `InTransitOnEdge`, set `LocatedIn = origin` (return to start), emit abort event
- `ActionDef` for Travel with:
  - Actor constraints: must be at origin place, must not already be in transit
  - Targets: destination place entity
  - Preconditions: travel edge exists between actor's current place and destination
  - Duration: from `TravelEdge.travel_time_ticks`
  - Body cost: configurable per-edge or fixed travel cost
  - Visibility: `VisibilitySpec` appropriate for departure/arrival witnesses

### 2. Register and export

## Files to Touch

- `crates/worldwake-systems/src/travel_actions.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module + export registration)
- `crates/worldwake-core/src/event_tag.rs` (modify — add Travel/Departure/Arrival event tags if needed)

## Out of Scope

- Pick-up / put-down actions (E10PROTRA-011)
- Multi-hop route planning (E13 AI)
- Ambush, escort, or interception logic (E12, future)
- Route danger scoring (explicitly forbidden by spec)
- Vehicle or mount systems (future)
- Pathfinding integration (exists in topology but not wired to actions here)

## Acceptance Criteria

### Tests That Must Pass

1. **Travel creates `InTransitOnEdge` for the full route duration** — component exists on actor for all intermediate ticks.
2. **Arrival removes `InTransitOnEdge` and updates `LocatedIn`** — actor is at destination after travel completes.
3. **Actor is NOT at origin during transit** — `LocatedIn` is cleared/removed at departure.
4. **Carried items remain with the carrier during transit** — ownership/containment relation unchanged.
5. **Travel fails if no edge connects origin and destination**.
6. **Travel fails if actor is already in transit** (has `InTransitOnEdge`).
7. **Travel duration matches `TravelEdge.travel_time_ticks`**.
8. **Aborted travel returns actor to origin** with `InTransitOnEdge` removed.
9. **Departure and arrival events emitted** with causal linkage.
10. **No teleportation**: actor cannot move to destination without traversing the edge.
11. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. No teleportation — goods and agents move only through physical travel.
2. An entity is either at a place (`LocatedIn`) or on an edge (`InTransitOnEdge`) — never both, never neither.
3. Carried items move because the carrier moves — no separate item movement.
4. Travel is deterministic given same inputs.
5. No floating-point arithmetic.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/travel_actions.rs` — happy path (depart→transit→arrive), abort, no-edge failure, already-in-transit failure, carried items, event emission, duration correctness

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
