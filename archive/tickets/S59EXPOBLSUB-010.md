# S59EXPOBLSUB-010: escort_to_safety action

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — new action in worldwake-systems, coupled multi-hop escort mechanism
**Deps**: S59EXPOBLSUB-002, S59EXPOBLSUB-005

## Problem

When an agent finds a wounded or incapacitated person during a search, they need to escort them to a safe place. This cannot be modeled honestly by having the actor travel first and only escort locally at the end, because that would split a single causal transport process into unrelated actions and strand the subject mid-route. The branch needs a coupled escort action that owns the full route from current place to destination and keeps actor + subject synchronized throughout.

## Assumption Reassessment (2026-04-07)

1. `crates/worldwake-systems/src/travel_actions.rs` currently owns single-actor direct-edge travel only. It cannot lawfully carry a missing subject through a multi-hop route by composition because `GoalKind::EscortToSafety { subject, destination }` already models escort as one continuous action surface, not `travel* + local handoff`.
2. `crates/worldwake-core/src/topology.rs` already provides deterministic `shortest_path()` routes with explicit edge sequences and total travel time. The missing piece is action/runtime state that can own that full route for both actor and subject.
3. `queue_for_care_target` at `crates/worldwake-systems/src/combat.rs:927-950,1464-1503` is a real handoff pattern, but it queues on the wounded patient entity itself. This ticket should reuse that contention pattern on arrival rather than inventing a destination-facility queue abstraction.
4. The branch does not currently expose a dedicated "care-capable place" belief substrate. This ticket should therefore validate a payload-bound believed destination and route, but it should not invent a new place-capability knowledge model in the same slice.
5. `S59EXPOBLSUB-009` already resolves overdue expectations on successful finds. This ticket must not claim ownership of expectation mutation for escorted subjects.
6. Authoritative wound/incapacitation checks already exist on the live action/belief surface; the stale `has_wounds()` wording in the draft should not be treated as a new API deliverable.

## Architecture Check

1. The novel boundary is not "single-edge dependent motion" but "route-aware coupled escort." The action should own the full route and keep actor + subject synchronized for its entire duration, including intermediate leg transitions and abort handling.
2. Preserve the carry/load model boundary. The subject is not converted into inventory or direct possession; escort is its own lawful movement relationship.
3. Reuse existing travel-style authoritative aftermath where applicable: in-transit state, carried-item movement with the actor, route evidence, and route experience. Do not fork a parallel transport semantics stack if the same invariants can be shared.
4. No backward compatibility shims.

## Verification Layers

1. Full-route escort moves both entities through one authoritative action → authoritative world state + active-action state
2. Arrival performs real care handoff through the existing patient contention path → authoritative queue state and/or action trace detail
3. Mid-route abort leaves both entities synchronized at the current route leg origin / last reached waypoint, never split across places → authoritative world state
4. Preconditions: co-location + wounded/incapacitated + reachable believed destination → focused unit test
5. Cross-system: topology routing + coupled transit + care handoff → action trace + event-log delta

## What to Change

### 1. Create escort_to_safety action

Create `crates/worldwake-systems/src/escort_actions.rs`:

- Domain: `ActionDomain::Care`
- Preconditions: Actor co-located with escortee. Escortee is wounded or incapacitated. Payload binds `{ subject, destination }`. Destination must be believed reachable and authoritatively route-reachable.
- Duration: Route travel action — total duration comes from the deterministic shortest path to the payload destination, not only a direct edge.
- on_start: Resolve and store the authoritative route. Begin the first leg for both actor and subject.
- on_tick: Advance through route legs, updating both actor and subject together at each reached waypoint, and keep both in transit on the current leg between waypoints.
- on_commit: Move both actor and escortee to final destination and perform real care handoff by reusing the existing `queue_for_care_target` / patient-contention pattern.
- on_abort: Clear in-transit state and leave both entities synchronized at the current leg origin / last reached waypoint.
- Affordance targets: co-located wounded/incapacitated agents
- Affordance payloads: enumerate destination candidates from the actor's currently believed places that are route-reachable on this branch; do not add a new place-capability belief model here.

### 2. Implement coupled multi-hop movement

During the escort action lifetime:
- Set escortee's `InTransitOnEdge` to mirror the actor's current route leg
- Advance both entities through each route waypoint atomically
- Move the actor's direct possessions with the actor as normal travel already does
- On commit: set both actor and escortee to the final destination
- On abort: return both to the current leg origin / last reached waypoint for this action state

This is authoritative coupled movement — the escortee does not independently choose travel actions, but they also are not converted into inventory/carry state.

### 3. Add the required runtime carriers

- Extend `worldwake-sim` action payload and trace detail for `escort_to_safety`
- Add route-aware escort local state and any required duration expression support
- Keep the surface compile-clean for later `S59EXPOBLSUB-011` goal emission

### 4. Register action

In `crates/worldwake-systems/src/action_registry.rs`, add `register_escort_to_safety_action()` and update completeness test.

## Files to Touch

- `crates/worldwake-systems/src/escort_actions.rs` (new)
- `crates/worldwake-systems/src/travel_actions.rs` or a shared helper surface if travel-state helpers need to be reused
- `crates/worldwake-systems/src/lib.rs` (modify — add module)
- `crates/worldwake-systems/src/action_registry.rs` (modify — register + test)
- `crates/worldwake-sim/src/action_payload.rs` (modify)
- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-sim/src/action_state.rs` and related duration support if required by the coupled route model

## Out of Scope

- AI goal emission for `EscortToSafety` — owned by `S59EXPOBLSUB-011`
- Inventing a new "care-capable place" belief/profile substrate
- New care-system behavior beyond reusing the existing patient contention / queue handoff pattern

## Acceptance Criteria

### Tests That Must Pass

1. Actor and escortee both arrive at a non-adjacent destination after the total route duration
2. Intermediate route progression keeps actor and escortee synchronized at reached waypoints and in transit on the same active leg
3. Arrival performs real care handoff via patient contention / queue state using the existing `queue_for_care_target` pattern
4. Escort aborted mid-route leaves actor and escortee synchronized at the current leg origin / last reached waypoint
5. Action rejected when escortee is not wounded/incapacitated
6. Action rejected when actor and escortee are not co-located
7. Action rejected when destination is not believed reachable / not authoritatively route-reachable
8. Action registry completeness test includes `escort_to_safety`
9. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Escortee's location always matches actor's escort path during the active escort action (no split)
2. Unique location invariant maintained — escortee has exactly one place at all times
3. Escort occupies actor's body (cannot perform other actions simultaneously, P8)
4. Both entities remain exposed to route danger/interruption during the full escorted route, not only the final edge

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/escort_actions.rs` — unit tests for travel, handoff, interruption, and preconditions
2. `crates/worldwake-systems/src/action_registry.rs` — updated completeness test
3. Any touched `worldwake-sim` payload/trace/state files — focused roundtrip or extraction tests as needed

### Commands

1. `cargo test -p worldwake-systems escort`
2. `cargo test -p worldwake-sim action_trace`
3. `cargo test -p worldwake-sim action_payload`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

- Completed on 2026-04-07.
- Corrected the ticket before implementation: the live branch did not have lawful multi-hop escort by composing existing single-actor `travel`, so this ticket was broadened to own a route-aware coupled escort action instead of a single-edge workaround.
- Added `EscortToSafetyActionPayload`, `ActionTraceDetail::EscortToSafety`, `ActionState::Escort`, and `DurationExpr::EscortRouteTravel` in `worldwake-sim`, plus belief/runtime duration support and the execution-context action-registry threading needed to resolve real care handoff ids from the live registry.
- Added `escort_to_safety` in `crates/worldwake-systems/src/escort_actions.rs`, registered it through the systems catalog, and reused existing travel/care helpers rather than forking parallel transport semantics.
- The landed action resolves a full deterministic route at start, keeps actor and subject synchronized through each leg with mirrored in-transit state, moves direct possessions with the actor, records route aftermath, and queues the escorter onto the wounded subject's existing care contention path on arrival.
- The ticket did not add new expectation mutation, destination-capability belief substrate, or AI goal emission; those remain outside the owned slice.

## Verification Result

- Passed `cargo test -p worldwake-systems escort -- --nocapture`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
