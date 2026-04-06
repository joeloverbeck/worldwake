# S59EXPOBLSUB-010: escort_to_safety action

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — new action in worldwake-systems, co-located dependent travel mechanism
**Deps**: S59EXPOBLSUB-002, S59EXPOBLSUB-005

## Problem

When an agent finds a wounded or incapacitated person during a search, they need to escort them to a safe place (e.g., a settlement with care facilities). This requires a travel action that moves two entities simultaneously — a co-located dependent travel mechanism that doesn't currently exist.

## Assumption Reassessment (2026-04-06)

1. Travel actions at `crates/worldwake-systems/src/travel_actions.rs` move a single entity along a topology edge. `escort_to_safety` needs to move actor + escortee together.
2. `queue_for_care_target` action at `crates/worldwake-systems/src/combat.rs:130-148` provides the care handoff pattern at destination.
3. `ActionDomain::Care` exists at `crates/worldwake-core/src/action_domain.rs:14`.
4. Items move with their holder during travel (carried items). The escortee mechanism is analogous — the escortee's location follows the actor during the travel action.
5. The escortee must be co-located, wounded or incapacitated. World API provides `has_wounds()` and `is_incapacitated()` checks.
6. The actor needs a "safe destination" in their beliefs — a place they believe has care facilities. This comes from `GoalBeliefView` knowledge about place capabilities.

## Architecture Check

1. The co-located dependent travel is the novel element. Two approaches: (a) actor moves and escortee position is updated atomically on commit, or (b) escortee is temporarily "carried" as a dependent entity. Approach (a) is simpler and avoids entangling with the carry/load system. Recommend (a).
2. No backward compatibility shims.

## Verification Layers

1. Both entities arrive at destination → authoritative world state (location check)
2. Escortee handed off to care system → action trace (queue_for_care_target triggered or care component updated)
3. Escort interrupted mid-route → both entities at intermediate place → authoritative world state
4. Preconditions: co-location + wounded/incapacitated → focused unit test
5. Cross-system: travel (topology) + care (handoff) → action trace + event-log delta

## What to Change

### 1. Create escort_to_safety action

Create `crates/worldwake-systems/src/escort_actions.rs`:

- Domain: `ActionDomain::Care`
- Preconditions: Actor co-located with escortee. Escortee is wounded or incapacitated. Actor knows a safe destination (place with care facilities in beliefs).
- Duration: Travel action — uses existing travel duration computation based on edge weights
- on_start: Record escortee entity. Begin travel toward destination.
- on_tick: Actor and escortee both in transit on the same edge. If interrupted, both land at the intermediate place.
- on_commit: Move both actor and escortee to destination. Trigger care handoff — set up conditions for the existing `queue_for_care_target` action to fire.
- on_abort: Both entities remain at current location (intermediate or origin).
- Affordance targets: co-located wounded/incapacitated agents
- Affordance payloads: enumerate destination candidates from actor's beliefs about safe places

### 2. Implement co-located dependent movement

During the travel phase (on_tick):
- Set escortee's `InTransitOnEdge` to match actor's transit state
- On commit: set escortee's location to destination (same as actor)
- On abort: set escortee's location to actor's current location

This is a direct location mutation — the escortee doesn't independently decide to travel. The escort action governs both entities' positions for its duration.

### 3. Register action

In `crates/worldwake-systems/src/action_registry.rs`, add `register_escort_to_safety_action()` and update completeness test.

## Files to Touch

- `crates/worldwake-systems/src/escort_actions.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module)
- `crates/worldwake-systems/src/action_registry.rs` (modify — register + test)

## Out of Scope

- Multi-hop escort (traveling through multiple edges) — uses existing travel infrastructure for multi-hop routing
- Escort failure due to route danger — the existing combat/interruption system handles encounters during travel
- care system modifications — uses existing queue_for_care_target as-is

## Acceptance Criteria

### Tests That Must Pass

1. Actor and escortee both arrive at destination after travel duration
2. Escortee is in care-ready state at destination (handoff to care queue)
3. Escort interrupted mid-route: both entities at intermediate place
4. Action rejected when escortee is not wounded/incapacitated
5. Action rejected when actor and escortee not co-located
6. Actor's ExpectationRecord updated if escortee was a search subject
7. Action registry completeness test includes "escort_to_safety"
8. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Escortee's location always matches actor's during escort (no split)
2. Unique location invariant maintained — escortee has exactly one place at all times
3. Escort occupies actor's body (cannot perform other actions simultaneously, P8)
4. Both entities exposed to route dangers during travel (P10 aftermath)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/escort_actions.rs` — unit tests for travel, handoff, interruption, and preconditions
2. `crates/worldwake-systems/src/action_registry.rs` — updated completeness test

### Commands

1. `cargo test -p worldwake-systems escort`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
