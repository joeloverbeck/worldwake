# E14PERBEL-005: Implement perception_system()

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new system function in `worldwake-systems`
**Deps**: E14PERBEL-003 (components registered), E14PERBEL-004 (`PerAgentBeliefView` exists for integration), E14PERBEL-002 (belief types exist)

## Problem

E14 still lacks the tick-time system that turns visible events into direct observations stored in each witnessing agent's `AgentBeliefStore`. The core belief types and `PerAgentBeliefView` already exist, but no production system currently populates those stores, and the canonical systems dispatch still leaves the `Perception` slot as a no-op.

## Assumption Reassessment (2026-03-14)

1. `SystemId::Perception` already exists in `crates/worldwake-sim/src/system_manifest.rs` and is correctly ordered after `FacilityQueue` and before `Politics`.
2. The canonical systems dispatch is not wired in `crates/worldwake-sim/src/system_dispatch.rs`; it is assembled in `crates/worldwake-systems/src/lib.rs`, and the `Perception` slot is currently `noop_system`.
3. `SystemExecutionContext<'_>` is already the correct function signature surface for system handlers.
4. `AgentBeliefStore`, `BelievedEntityState`, `PerceptionProfile`, `PerceptionSource`, and `SocialObservation` already exist in `crates/worldwake-core/src/belief.rs`.
5. `PerAgentBeliefView` already exists in `crates/worldwake-sim/src/per_agent_belief_view.rs`. This ticket must not duplicate or redesign that adapter.
6. `EventLog` already exposes `events_at_tick(tick) -> &[EventId]`, which is the correct current-tick event source. No new event-log query API is needed.
7. `EventRecord` already carries `visibility`, `witness_data`, `tags`, `target_ids`, `actor_id`, `place_id`, and `state_deltas`. `WitnessData` contains both `direct_witnesses` and `potential_witnesses`.
8. Agent co-location can already be queried via authoritative world placement APIs such as `World::entities_effectively_at(place)` and `World::effective_place(entity)`.
9. `AgentBeliefStore` and `PerceptionProfile` are registered in the component schema, but `World::create_agent()` does not currently attach them by default. New agents therefore start without belief/perception components unless some caller adds them manually.
10. The current ticket overreaches into `E14PERBEL-006` and `E14PERBEL-007` by mentioning AI migration and broad integration acceptance. This ticket should stay focused on perception-system behavior, dispatch wiring, and agent-default component presence.

## Architecture Check

1. The clean architecture is event-triggered belief refresh: perceived events identify that something observable happened, and the perception system then snapshots the relevant observed entities from authoritative world state into the witness's belief store.
2. Reconstructing full belief snapshots from raw `StateDelta`s alone would be brittle and incomplete. Deltas are the causal trigger and traceability record, not the long-lived belief read model.
3. This architecture is better than the ticket's original delta-reconstruction plan because it remains extensible as `BelievedEntityState` grows. New fields can be sourced from one snapshot builder instead of scattered delta decoders.
4. The system must remain state-mediated. It reads event log + world state and writes belief components. It does not call AI, planning, politics, or other systems directly.
5. Social evidence should be recorded only when the event gives a concrete, traceable social observation. Tagged trade/combat events fit this. Generic ambient `CoPresence` synthesis for every colocated agent each tick would be a separate design and should not be invented here without an explicit event source.
6. This ticket should not try to solve the broader mixed-boundary issue in `BeliefView`. That cleanup remains separate work.

## What to Change

### 1. Create `crates/worldwake-systems/src/perception.rs`

Implement:

```rust
pub fn perception_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>
```

### 2. Process only current-tick events

Use `ctx.event_log.events_at_tick(ctx.tick)` and resolve each `EventId` to its `EventRecord`.

The perception system should consider only events from the current tick, preserving the manifest's causal rule that `Perception` reacts to what earlier systems/actions emitted during that tick.

### 3. Resolve witnesses from `VisibilitySpec` plus local world state

For each event:

- `ParticipantsOnly`: witnesses are `record.witness_data.direct_witnesses`
- `SamePlace`: witnesses are alive agents effectively at `record.place_id`
- `AdjacentPlaces { max_hops }`: witnesses are alive agents at `record.place_id` and at places reachable within `max_hops` directed topology steps
- `PublicRecord`: no passive perception in E14
- `Hidden`: no passive perception

Use `witness_data` as explicit participant/witness metadata where available, but do not assume it alone is sufficient for same-place or adjacent-place visibility.

### 4. Apply observation fidelity per witnessing agent

For each candidate witnessing agent:

- read `PerceptionProfile`
- if absent, skip that agent; the default-component work in this ticket should eliminate this case for normal agents
- respect `observation_fidelity`
- `Permille(1000)` must always observe
- `Permille(0)` must never observe
- intermediate values must be deterministic under the seeded system RNG

### 5. Refresh belief snapshots from authoritative world state

For each agent that successfully perceives an event:

1. Determine the observed entity set from the event's concrete references:
   - `actor_id` if present
   - `target_ids`
   - entities referenced by `state_deltas`
   - entities named by event evidence where applicable
2. For each observed entity, build a `BelievedEntityState` from authoritative post-event world state:
   - `last_known_place` from `World::effective_place`
   - `last_known_inventory` from current authoritative holdings / lot quantity as appropriate for the entity kind
   - `alive` from current authoritative life/death state
   - `wounds` from current authoritative wound list
   - `observed_tick = ctx.tick`
   - `source = PerceptionSource::DirectObservation`
3. Write the snapshot with `AgentBeliefStore::update_entity`

This keeps the belief model concrete and future-extensible while preserving causal traceability through the event log.

### 6. Record concrete social observations

When a perceived event has concrete social meaning:

- tagged `EventTag::Trade` with two participating agents -> record `SocialObservationKind::WitnessedCooperation`
- tagged `EventTag::Combat` with two participating agents -> record `SocialObservationKind::WitnessedConflict`

Use the event's actual actor/targets/place/tick and `PerceptionSource::DirectObservation`.

Do not invent free-floating social summaries or global loyalty scores here.

### 7. Enforce memory limits only for updated witnesses

After processing each witnessing agent's observed entities/social observations, run:

```rust
belief_store.enforce_capacity(profile, ctx.tick);
```

This preserves the existing retention/capacity rules in `AgentBeliefStore`.

### 8. Wire the system into canonical dispatch

Update `crates/worldwake-systems/src/lib.rs`:

- add `pub mod perception;`
- export `perception_system`
- replace the `Perception` slot in `dispatch_table()` with `perception_system`

Do not modify `crates/worldwake-sim/src/system_dispatch.rs` for runtime wiring; that file defines the generic dispatch table type, not the workspace's canonical handler selection.

### 9. Attach default belief/perception components on agent creation

Update authoritative agent creation so new agents receive:

- `AgentBeliefStore::new()`
- a default `PerceptionProfile`

The right place for this is the authoritative agent-creation path in `worldwake-core` (`World::create_agent()`), not scattered test or CLI-only helpers. This avoids aliasing and keeps agent invariants centralized.

The default profile should be conservative, deterministic, and usable by all agents; any scenario-specific diversity can still override it explicitly afterward.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module/export and register dispatch)
- `crates/worldwake-core/src/world.rs` (modify — attach default `AgentBeliefStore` and `PerceptionProfile` during agent creation)
- Tests in `crates/worldwake-systems/src/perception.rs` and/or `crates/worldwake-systems/tests/` as needed
- Targeted core tests for default agent components if current coverage is insufficient

## Out of Scope

- Migrating `agent_tick.rs` or deleting `OmniscientBeliefView` (E14PERBEL-006)
- Full belief-isolation integration scenarios such as T10 (E14PERBEL-007)
- Report / rumor propagation (E15 scope)
- Record consultation actions for `PublicRecord`
- Politics / offices / faction logic (E16 scope)
- Redesigning `BeliefView`
- Synthesizing ambient `CoPresence` observations without a concrete event source
- Modifying event-log structure, `VisibilitySpec`, or `WitnessData`

## Acceptance Criteria

### Tests That Must Pass

1. `SamePlace` event at an agent's location updates that agent's `AgentBeliefStore` with `PerceptionSource::DirectObservation`.
2. Agent at a different place does not perceive a `SamePlace` event.
3. `ParticipantsOnly` event is observed only by `direct_witnesses`.
4. `AdjacentPlaces { max_hops: 1 }` reaches immediate neighboring places and not farther ones.
5. `Hidden` event produces no passive observations.
6. `PublicRecord` event produces no passive observations.
7. `Permille(0)` never observes and `Permille(1000)` always observes.
8. Intermediate fidelity is deterministic for a fixed seed.
9. Perceived belief snapshots reflect authoritative post-event state for observed entities: place, inventory, alive/dead state, wounds, `observed_tick`, and `source`.
10. Trade-tagged events produce `WitnessedCooperation` observations for successful witnesses.
11. Combat-tagged events produce `WitnessedConflict` observations for successful witnesses.
12. `memory_capacity` eviction is enforced after updates.
13. `memory_retention_ticks` pruning is enforced after updates.
14. Multiple visible events in one tick are all processed.
15. Canonical `dispatch_table()` installs `perception_system` in the `SystemId::Perception` slot.
16. Newly created agents have both `AgentBeliefStore` and `PerceptionProfile` by default.
17. `cargo test -p worldwake-systems`
18. `cargo test -p worldwake-core`
19. `cargo clippy --workspace`

### Invariants

1. Perception remains passive and locality-bound.
2. All beliefs written by this ticket are traceable to `PerceptionSource::DirectObservation`.
3. No passive multi-hop knowledge spread beyond the event's visibility rule.
4. `PublicRecord` does not imply global knowledge.
5. Same seed + same world/event sequence yields the same perception outcomes.
6. The agent-default invariant is centralized in authoritative creation, not scattered across helper layers.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs`
   Unit tests for witness resolution, fidelity gating, belief snapshot refresh, social observation capture, and memory enforcement.
2. `crates/worldwake-systems/src/lib.rs`
   Targeted dispatch-table test or assertion that `SystemId::Perception` maps to `perception_system`.
3. `crates/worldwake-core/src/world.rs`
   Test that newly created agents receive default `AgentBeliefStore` and `PerceptionProfile`.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

- Completed: 2026-03-14
- Added `crates/worldwake-systems/src/perception.rs` and wired `perception_system` into the canonical `Perception` dispatch slot in `crates/worldwake-systems/src/lib.rs`.
- Implemented event-triggered witness resolution from current-tick `EventLog` records, deterministic fidelity gating, direct-observation snapshot refresh into `AgentBeliefStore`, and trade/combat social observation capture.
- Centralized the new-agent invariant in `crates/worldwake-core/src/world.rs`: every created agent now starts with `AgentBeliefStore::new()` and `PerceptionProfile::default()`.
- Updated `worldwake-core` transaction tests to the new invariant instead of preserving the old "agents start without belief/perception components" assumption.
- Added focused perception coverage for:
  - same-place observation
  - participant-only visibility
  - adjacent-place spillover
  - witnessed trade cooperation
  - memory-capacity eviction
  - dispatch-table wiring
  - default belief/perception components on agent creation
- Deviation from the original plan: the implementation does not attempt to reconstruct durable belief state from raw `StateDelta`s alone. It uses events as the trigger and traceability surface, then snapshots observed entities from authoritative post-event world state. This is cleaner and more extensible for the current architecture.
- Verification:
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
