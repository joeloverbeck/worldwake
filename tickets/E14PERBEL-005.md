# E14PERBEL-005: Implement perception_system()

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new system function in worldwake-systems
**Deps**: E14PERBEL-003 (components registered), E14PERBEL-004 (PerAgentBeliefView exists for integration testing)

## Problem

A `perception_system()` must run each tick to process events, resolve witnesses per `VisibilitySpec` and co-location rules, and update each witnessing agent's `AgentBeliefStore`. Without this system, belief stores remain empty and agents have no perceived information.

## Assumption Reassessment (2026-03-14)

1. `SystemId::Perception` is already defined in `system_manifest.rs` at ordinal 5, running after `FacilityQueue` and before `Politics` — confirmed.
2. System functions receive `SystemExecutionContext` — check actual type. Other systems like `needs_system()` and `run_combat_system()` show the signature pattern.
3. Events are in the append-only `EventLog` with `EventRecord` entries that contain `WitnessData` and `VisibilitySpec` — confirmed.
4. `WitnessData` has `direct_witnesses: BTreeSet<EntityId>` and `potential_witnesses: BTreeSet<EntityId>` — confirmed.
5. `VisibilitySpec` has 5 variants: `ParticipantsOnly`, `SamePlace`, `AdjacentPlaces { max_hops }`, `PublicRecord`, `Hidden` — confirmed.
6. The perception system needs to process events emitted during the current tick — need to identify how to get current-tick events from EventLog.
7. Agent co-location: agents at the same place as the event can perceive `SamePlace` events — check how place membership is queried.

## Architecture Check

1. System lives in `worldwake-systems` crate, consistent with `needs_system()`, `run_combat_system()`, etc.
2. System reads events from EventLog (read-only), reads World for agent locations (read-only for co-location check), and writes to `AgentBeliefStore` components (mutation).
3. Cross-system interaction is state-mediated: perception reads events written by all prior systems, writes beliefs consumed by AI.

## What to Change

### 1. Create `crates/worldwake-systems/src/perception.rs`

Implement:

```rust
pub fn perception_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>
```

**Algorithm per tick:**

1. Collect events emitted during the current tick from the event log.
2. For each event:
   a. Read the event's `VisibilitySpec` and `WitnessData`.
   b. Determine which agents can perceive this event:
      - `ParticipantsOnly`: only agents listed in `WitnessData.direct_witnesses`
      - `SamePlace`: all agents at the event's location
      - `AdjacentPlaces { max_hops }`: agents at the event's place OR within `max_hops` adjacent places (immediate spillover only per FND-01 Section B)
      - `PublicRecord`: no passive perception — agents must travel to consult (E14 does not implement consultation action; just skip for now)
      - `Hidden`: no perception
   c. For each perceiving agent:
      - Check `PerceptionProfile.observation_fidelity` — roll against seeded RNG to determine if the agent notices. If fidelity is 1000 (Permille max), always notice.
      - If noticed: extract relevant entity state changes from the event and update the agent's `AgentBeliefStore`:
        - Entity location changes → update `last_known_place`
        - Inventory changes → update `last_known_inventory`
        - Death → set `alive = false`
        - Wounds → update wounds list
        - Set `observed_tick = current_tick`
        - Set `source = PerceptionSource::DirectObservation`
      - Record social observations for cooperation/conflict events (trade = cooperation, combat = conflict, co-presence for agents at same place)
   d. After processing, enforce `memory_capacity` and `memory_retention_ticks` for each updated agent's belief store.

### 2. Register system dispatch

Wire `perception_system` into `SystemDispatch` so it executes at the `SystemId::Perception` slot. Follow the pattern used by `needs_system`, `run_combat_system`, etc. in `system_dispatch.rs`.

### 3. Initialize belief stores on agent creation

Ensure all agents get an empty `AgentBeliefStore` and a default `PerceptionProfile` when created. Check agent creation paths (likely in `test_utils` and CLI agent creation). This may be handled in E14PERBEL-006 migration instead — decide based on where agents are created.

### 4. Register module in `crates/worldwake-systems/src/lib.rs`

Add `pub mod perception;` and re-export `perception_system`.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module)
- `crates/worldwake-sim/src/system_dispatch.rs` (modify — register perception system dispatch)
- `crates/worldwake-core/src/test_utils.rs` (modify — add default PerceptionProfile + AgentBeliefStore to test agent creation helpers if they exist)

## Out of Scope

- Report/rumor perception (E15 scope — agents telling each other things)
- Record consultation action (E15+ scope)
- `PublicRecord` consultation mechanics (future epic)
- Modifying event emission in other systems (they already emit events correctly)
- Changing `VisibilitySpec` enum variants
- Changing `WitnessData` struct
- Modifying `EventLog` or `EventRecord` structures
- Migrating `agent_tick.rs` (E14PERBEL-006)
- Politics system (E16)

## Acceptance Criteria

### Tests That Must Pass

1. Agent at same place as event perceives it — `AgentBeliefStore` updated with `DirectObservation` source
2. Agent at different place does NOT perceive `SamePlace` event — belief store unchanged
3. `ParticipantsOnly` event: only direct witnesses perceive
4. `AdjacentPlaces { max_hops: 1 }` event: agents at adjacent places perceive, agents 2+ hops away do not
5. `Hidden` event: no agent perceives (even co-located)
6. `PublicRecord` event: no passive perception (agents must actively consult — not implemented in E14)
7. `observation_fidelity` check: agent with `Permille(0)` never perceives; agent with `Permille(1000)` always perceives
8. `observation_fidelity` check: agent with intermediate fidelity sometimes perceives (deterministic with seeded RNG)
9. Perceived entity state matches event data: location, inventory changes, death, wounds
10. `observed_tick` set to current tick on perception
11. Social observation recorded for trade event (WitnessedCooperation)
12. Social observation recorded for combat event (WitnessedConflict)
13. `memory_capacity` enforced: oldest entries evicted when store exceeds capacity
14. `memory_retention_ticks` enforced: entries older than retention removed
15. Multiple events in one tick: all processed, all perceiving agents updated
16. System runs at correct position in tick order (after FacilityQueue, before Politics)
17. `cargo test -p worldwake-systems`
18. `cargo clippy --workspace`

### Invariants

1. Perception is passive — agents don't choose what to notice (spec requirement)
2. All beliefs traceable to `DirectObservation` source (FND-01 Section B requirement 4)
3. No instant multi-hop information spread (FND-01 Section B requirement 1)
4. `AdjacentPlaces` limited to immediate physical spillover (FND-01 Section B requirement 3)
5. `PublicRecord` does NOT grant global knowledge (FND-01 Section B requirement 2)
6. Deterministic: same seed + same events → same perception outcomes
7. System decoupling: perception system reads events/world state, never calls other systems directly (Principle 12)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` — unit tests for visibility resolution, fidelity rolls, belief updates
2. Integration tests in `crates/worldwake-systems/tests/` — multi-agent scenarios with different locations and event types

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
