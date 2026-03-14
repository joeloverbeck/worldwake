# E15RUMWITDIS-009: EntityMissing Detection and Event-Based Mismatch Detection

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — EntityMissing detection in perception, event-based mismatch comparison
**Deps**: `archive/tickets/completed/E15RUMWITDIS-008.md`, `archive/tickets/completed/E15RUMWITDIS-004.md`, `specs/E15-rumor-witness-discovery.md`

## Problem

Two mismatch detection paths remain after E15RUMWITDIS-008:

1. **EntityMissing**: After passive observation at a place, the agent expected an entity there (prior belief `last_known_place == current_place`) but the entity was NOT observed. This requires comparing the set of "believed to be here" entities against the set of "actually observed here" entities.

2. **Event-based perception mismatch**: When the perception system processes event log entries and updates beliefs from witnessed events, the same mismatch comparison (AliveStatusChanged, InventoryDiscrepancy, PlaceChanged) should apply — not just passive observation.

## Assumption Reassessment (2026-03-14)

1. `observe_passive_local_entities()` iterates `world.entities_effectively_at(place)` for each observer — this gives the "actually present" set.
2. `AgentBeliefStore.known_entities` keys filtered by `last_known_place == observer's place` gives the "believed to be here" set.
3. The event-based perception path in `perception_system()` currently calls `build_believed_entity_state()` and `store.update_entity()` directly for event-observed entities. It does NOT yet route through the passive mismatch helper path, so the ticket must correct that rather than assuming it is already shared.
4. Event-based perception currently snapshots entities from the authoritative world state at the time `perception_system()` runs, not from a reconstructed event-local pre/post snapshot. This ticket should preserve that architecture and only compare beliefs against the post-event observed snapshot already used by the perception system.
5. `E15RUMWITDIS-008` is now completed and archived. It already introduced passive mismatch detection and Discovery emission helpers in `crates/worldwake-systems/src/perception.rs`, but those helpers are still passive-specific today.
6. `MismatchKind`, `EvidenceRef::Mismatch`, and `EventTag::Discovery` are already implemented and in use. This ticket extends where those primitives are emitted; it does not introduce new event or evidence infrastructure.

## Architecture Check

1. EntityMissing is a post-observation check: after all entities at the place are observed, scan beliefs for entities believed-at-this-place that were NOT in the observed set.
2. Event-based mismatch should reuse and, if necessary, generalize the same comparison and Discovery event-construction path introduced in `E15RUMWITDIS-008`. This ticket should not create a second parallel mismatch implementation.
3. EntityMissing uses the observation_fidelity check: if fidelity filtered out the entity (random check failed), do NOT report EntityMissing for it. The agent "didn't notice" rather than "noticed absence."
4. Event-based `PlaceChanged` detection is only honest when the new observed snapshot itself carries a different `last_known_place` than the prior belief. This ticket must not attempt a larger event-snapshot reconstruction layer from raw `state_deltas`.
5. Because passive same-place observation already runs earlier in the tick, event-based mismatch coverage mainly matters for witnesses who learn through event visibility rather than passive co-location. Tests should exercise those visibility-driven cases, not duplicate passive coverage.
6. No backwards-compatibility shims.

## What to Change

### 1. Generalize the existing mismatch helper path

Start from the helpers added in `E15RUMWITDIS-008` and extend them so both passive and event-based perception share the same mismatch comparison and Discovery event construction path.

If the current helper names or signatures are too passive-specific, rename or widen them in place rather than adding parallel helpers. The shared comparison should support "include place mismatch" as an explicit capability rather than creating a second event-only comparison stack.

### 2. Add EntityMissing detection

After passive observation completes for an observer at a place, scan the agent's beliefs:

1. Collect all entities the agent believes are at this place: `known_entities.iter().filter(|(_id, belief)| belief.last_known_place == Some(current_place))`.
2. Subtract the set of entities actually observed this tick (tracked during the observation loop).
3. For each "believed here but not seen" entity, emit a Discovery event with `MismatchKind::EntityMissing`.
4. Important: only report EntityMissing for entities that passed (or would have passed) the observation_fidelity check — if the agent's fidelity is too low, they can't reliably notice absence.

### 3. Add event-based mismatch detection

In the event-based perception loop (main `perception_system()` function), before calling `store.update_entity(entity, snapshot)`, call the shared mismatch comparison helper path from step 1. Emit Discovery events for any detected mismatches.

Important scope clarification:

1. Compare against the same post-event snapshot that event perception already writes today.
2. Do NOT add event-local snapshot reconstruction from `EventRecord.state_deltas` in this ticket.
3. Event-based tests should use `ParticipantsOnly` or `AdjacentPlaces` visibility so they verify behavior that passive same-place observation would not already cover earlier in the tick.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify — generalize existing mismatch helpers, add EntityMissing detection, add event-based mismatch detection)

## Out of Scope

- Tell action or TellProfile
- AI investigation goal generation from Discovery events (E17)
- Crime evidence gathering (E17)
- PlaceChanged detection in passive observation (entities at same place don't change place from the observer's perspective — PlaceChanged only applies in event-based perception)
- Reconstructing per-event entity snapshots from `EventRecord.state_deltas` or preserving intermediate same-tick entity states for perception
- belief_confidence() function (E15RUMWITDIS-010)

## Acceptance Criteria

### Tests That Must Pass

1. Discovery event emitted on EntityMissing (agent believed entity at place, entity not found via passive observation)
2. No EntityMissing for entities the agent has no prior belief about
3. No EntityMissing if entity is still at the place (false alarm prevention)
4. Event-based perception detects `AliveStatusChanged` mismatch for a visibility-driven witness path that passive same-place observation would not already satisfy
5. Event-based perception detects `InventoryDiscrepancy` mismatch for a visibility-driven witness path that passive same-place observation would not already satisfy
6. `PlaceChanged` detection in the event-based path is emitted when the post-event observed snapshot reveals a concrete place different from the prior belief
7. Multiple missing entities produce multiple Discovery events
8. Existing perception tests continue to pass
9. Existing suite: `cargo test --workspace`
10. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. EntityMissing requires a prior belief with `last_known_place == observer's place`
2. EntityMissing is not triggered for entities the agent never believed were at this place
3. Mismatch comparison and Discovery event construction remain shared between passive and event-based paths (no duplicated helper stacks)
4. Discovery events are always VisibilitySpec::ParticipantsOnly

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs::tests` — test: agent believed entity at place, entity moved away, passive observation emits EntityMissing
2. `crates/worldwake-systems/src/perception.rs::tests` — test: agent has no belief about entity at place, entity absent, no EntityMissing
3. `crates/worldwake-systems/src/perception.rs::tests` — test: event-based `ParticipantsOnly` or `AdjacentPlaces` perception updates belief with `AliveStatusChanged` mismatch, Discovery emitted
4. `crates/worldwake-systems/src/perception.rs::tests` — test: event-based visibility-driven perception updates belief with `InventoryDiscrepancy` or `PlaceChanged`, Discovery emitted
5. `crates/worldwake-systems/src/perception.rs::tests` — test: event-based perception updates belief with no mismatch, no Discovery emitted

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - Corrected the ticket first so it matched the live code: the event path was not yet sharing passive mismatch helpers, and event perception still reads post-event authoritative snapshots rather than reconstructed event-local snapshots.
  - Generalized the perception mismatch path in `crates/worldwake-systems/src/perception.rs` so passive observation and event-based perception both emit Discovery events through the same helper flow.
  - Added passive `MismatchKind::EntityMissing` detection after local observation completes, with fidelity gating to avoid false missing discoveries from entities that were simply not noticed.
  - Added event-based mismatch emission for `AliveStatusChanged`, `InventoryDiscrepancy`, and `PlaceChanged` using the same post-event snapshot path the perception system already writes into belief memory.
  - Added regression tests covering passive missing-entity discovery, false-positive prevention, event-based alive/inventory/place mismatches, and a no-mismatch event path.
- Deviations from original plan:
  - This ticket did not add a new event-local snapshot reconstruction layer from `EventRecord.state_deltas`. That would be a broader architectural change than the existing perception system uses today, so the implementation stayed aligned with the current post-event snapshot model and the ticket scope was corrected accordingly before code changes.
  - Event-based mismatch coverage was tested through visibility-driven paths (`AdjacentPlaces`) so it exercises behavior that passive same-place observation does not already satisfy earlier in the tick.
- Verification results:
  - `cargo test -p worldwake-systems perception`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
