# E15RUMWITDIS-009: EntityMissing Detection and Event-Based Mismatch Detection

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — EntityMissing detection in perception, event-based mismatch comparison
**Deps**: E15RUMWITDIS-008 (passive observation mismatch detection infrastructure)

## Problem

Two mismatch detection paths remain after E15RUMWITDIS-008:

1. **EntityMissing**: After passive observation at a place, the agent expected an entity there (prior belief `last_known_place == current_place`) but the entity was NOT observed. This requires comparing the set of "believed to be here" entities against the set of "actually observed here" entities.

2. **Event-based perception mismatch**: When the perception system processes event log entries and updates beliefs from witnessed events, the same mismatch comparison (AliveStatusChanged, InventoryDiscrepancy, PlaceChanged) should apply — not just passive observation.

## Assumption Reassessment (2026-03-14)

1. `observe_passive_local_entities()` iterates `world.entities_effectively_at(place)` for each observer — this gives the "actually present" set.
2. `AgentBeliefStore.known_entities` keys filtered by `last_known_place == observer's place` gives the "believed to be here" set.
3. The event-based perception path (the main loop in `perception_system()`) calls `build_believed_entity_state()` and `store.update_entity()` — same pattern as passive observation, needs same mismatch comparison.
4. E15RUMWITDIS-008 may or may not have extracted reusable mismatch helpers. If it did not, this ticket must extract them before extending behavior.

## Architecture Check

1. EntityMissing is a post-observation check: after all entities at the place are observed, scan beliefs for entities believed-at-this-place that were NOT in the observed set.
2. Event-based mismatch reuses the exact same comparison and Discovery event-construction logic as passive observation. Shared helper extraction is required here if `E15RUMWITDIS-008` did not already do it.
3. EntityMissing uses the observation_fidelity check: if fidelity filtered out the entity (random check failed), do NOT report EntityMissing for it. The agent "didn't notice" rather than "noticed absence."
4. No backwards-compatibility shims.

## What to Change

### 1. Extract mismatch comparison helper

Extract the mismatch comparison logic and Discovery event construction into reusable private helpers in `perception.rs` before adding the remaining behaviors. This is required to avoid passive/event-based drift.

```rust
fn detect_mismatches(
    prior: &BelievedEntityState,
    new: &BelievedEntityState,
) -> Vec<MismatchKind> { ... }

fn emit_discovery_events(
    event_log: &mut EventLog,
    tick: Tick,
    observer: EntityId,
    place: EntityId,
    subject: EntityId,
    mismatches: Vec<MismatchKind>,
) { ... }
```

### 2. Add EntityMissing detection

After passive observation completes for an observer at a place, scan the agent's beliefs:

1. Collect all entities the agent believes are at this place: `known_entities.iter().filter(|(_id, belief)| belief.last_known_place == Some(current_place))`.
2. Subtract the set of entities actually observed this tick (tracked during the observation loop).
3. For each "believed here but not seen" entity, emit a Discovery event with `MismatchKind::EntityMissing`.
4. Important: only report EntityMissing for entities that passed (or would have passed) the observation_fidelity check — if the agent's fidelity is too low, they can't reliably notice absence.

### 3. Add event-based mismatch detection

In the event-based perception loop (main `perception_system()` function), before calling `store.update_entity(entity, snapshot)`, call the same mismatch comparison helper from step 1. Emit Discovery events for any detected mismatches.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify — extract mismatch helper, add EntityMissing detection, add event-based mismatch detection)

## Out of Scope

- Tell action or TellProfile
- AI investigation goal generation from Discovery events (E17)
- Crime evidence gathering (E17)
- PlaceChanged detection in passive observation (entities at same place don't change place from the observer's perspective — PlaceChanged only applies in event-based perception)
- belief_confidence() function (E15RUMWITDIS-010)

## Acceptance Criteria

### Tests That Must Pass

1. Discovery event emitted on EntityMissing (agent believed entity at place, entity not found via passive observation)
2. No EntityMissing for entities the agent has no prior belief about
3. No EntityMissing if entity is still at the place (false alarm prevention)
4. Event-based perception detects AliveStatusChanged mismatch
5. Event-based perception detects InventoryDiscrepancy mismatch
6. PlaceChanged detection in event-based path: belief says place A, event reveals place B
7. Multiple missing entities produce multiple Discovery events
8. Existing perception tests continue to pass
9. Existing suite: `cargo test --workspace`
10. `cargo clippy --workspace`

### Invariants

1. EntityMissing requires a prior belief with `last_known_place == observer's place`
2. EntityMissing is not triggered for entities the agent never believed were at this place
3. Mismatch comparison and Discovery event construction are shared between passive and event-based paths (no duplication)
4. Discovery events are always VisibilitySpec::ParticipantsOnly

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs::tests` — test: agent believed entity at place, entity moved away, passive observation emits EntityMissing
2. `crates/worldwake-systems/src/perception.rs::tests` — test: agent has no belief about entity at place, entity absent, no EntityMissing
3. `crates/worldwake-systems/src/perception.rs::tests` — test: event-based perception updates belief with mismatch, Discovery emitted
4. `crates/worldwake-systems/src/perception.rs::tests` — test: event-based perception updates belief with no mismatch, no Discovery emitted

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
