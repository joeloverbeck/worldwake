# E15RUMWITDIS-008: Belief Mismatch Detection in Passive Observation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — mismatch detection in perception system, Discovery event emission
**Deps**: E15RUMWITDIS-002 (EventTag::Discovery), E15RUMWITDIS-004 (MismatchKind, EvidenceRef::Mismatch)

## Problem

When the perception system updates a belief and detects material mismatch between the prior belief and the new observation, it must emit a Discovery event into the append-only event log. This is the Principle 15 foundation: surprise comes from violated expectation. Currently `observe_passive_local_entities()` overwrites beliefs without comparing old vs new.

## Assumption Reassessment (2026-03-14)

1. `observe_passive_local_entities()` in `crates/worldwake-systems/src/perception.rs` (line 88) — confirmed. Currently iterates entities at same place, builds snapshots via `build_believed_entity_state()`, and calls `store.update_entity()` without any prior-belief comparison.
2. `AgentBeliefStore.get_entity()` returns `Option<&BelievedEntityState>` — can read prior belief before overwrite.
3. `MismatchKind` enum (from E15RUMWITDIS-004) covers: EntityMissing, AliveStatusChanged, InventoryDiscrepancy, PlaceChanged.
4. Discovery events use `VisibilitySpec::ParticipantsOnly` (private to the observer) per spec.
5. The perception system commits belief updates via a single WorldTxn at the end — Discovery events should be emitted into the event log before/alongside this commit.

## Architecture Check

1. Mismatch comparison happens BEFORE `update_entity()` overwrites the prior belief. We read the old belief, compare with new snapshot, emit Discovery if different, then update.
2. Discovery events are emitted into the event_log directly (not via the belief-update WorldTxn) — they are separate observable events with their own metadata.
3. Only material differences trigger Discovery — not every field change. Material = alive status, inventory quantities, place, entity presence.
4. This ticket may introduce a private helper for mismatch comparison and discovery-event construction if that keeps `E15RUMWITDIS-009` from duplicating logic. If the helper is not extracted here, `-009` must extract it instead of copying the code.
5. No backwards-compatibility shims.

## What to Change

### 1. Add mismatch comparison in `observe_passive_local_entities()`

In `crates/worldwake-systems/src/perception.rs`, modify `observe_passive_local_entities()`:

Before calling `store.update_entity(entity, snapshot)`, check if the agent already has a belief about this entity. If yes, compare:

- **AliveStatusChanged**: prior.alive ≠ new.alive
- **InventoryDiscrepancy**: for each commodity, prior quantity ≠ new quantity (emit one Discovery per commodity mismatch)
- **PlaceChanged**: prior.last_known_place ≠ new.last_known_place (only if both are Some)

For each detected mismatch, emit a Discovery event into `event_log` with:
- actor_id = observer
- place_id = observation place
- tags = {Discovery, WorldMutation}
- visibility = VisibilitySpec::ParticipantsOnly
- evidence = EvidenceRef::Mismatch { observer, subject: entity, kind: mismatch }

### 2. Handle "no prior belief" case

If the agent has no prior belief about the entity (first observation), do NOT emit a Discovery event — this is expected behavior, not a mismatch.

### 3. Pass event_log into `observe_passive_local_entities()`

Currently the function signature only takes `world`, `tick`, `rng`, `updated_stores`. Add `event_log: &mut EventLog` parameter so it can emit Discovery events.

### 4. Keep mismatch logic shareable

Do not hard-wire mismatch comparison or Discovery event construction into a passive-only code path in a way that `E15RUMWITDIS-009` would have to duplicate. Either:

- extract a private helper in this ticket, or
- keep the passive implementation small enough that `-009` can extract the helper as part of its work without behavioral churn.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify — add mismatch detection, emit Discovery events, update function signature)

## Out of Scope

- EntityMissing detection (agent expected entity at place but didn't find it) — that is E15RUMWITDIS-009
- Event-based perception mismatch detection (mismatch from witnessing events, not passive observation) — that is E15RUMWITDIS-009
- Tell action (E15RUMWITDIS-005/006/007)
- AI investigation goal generation from Discovery events (future work, E17)
- belief_confidence() function (E15RUMWITDIS-010)

## Acceptance Criteria

### Tests That Must Pass

1. Discovery event emitted on AliveStatusChanged (believed alive, observed dead)
2. Discovery event emitted on InventoryDiscrepancy (believed N, observed M)
3. Discovery event emitted on PlaceChanged (believed at place A, observed at place B) — this occurs in event-based perception (E15RUMWITDIS-009), not passive local observation (entities at same place don't change place from observer's perspective)
4. No Discovery event when agent has no prior belief (first observation)
5. Discovery event has VisibilitySpec::ParticipantsOnly
6. Discovery event tagged with {Discovery, WorldMutation}
7. Discovery event carries correct EvidenceRef::Mismatch evidence
8. Multiple mismatches on same entity produce multiple Discovery events (or one event with multiple evidence entries)
9. Existing perception tests continue to pass
10. Existing suite: `cargo test --workspace`
11. `cargo clippy --workspace`

### Invariants

1. Discovery events are append-only in the event log (never mutated)
2. Mismatch detection does not alter the belief update itself — beliefs still update normally
3. First observations never trigger Discovery (no prior belief = no expectation to violate)
4. Principle 15 (violated expectation): Discovery is emitted ONLY when prior belief exists AND differs materially
5. This ticket must not force duplicated mismatch/emission logic across passive and event-based perception paths

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs::tests` — test: agent has prior belief about alive target, target dies, passive observation emits Discovery with AliveStatusChanged
2. `crates/worldwake-systems/src/perception.rs::tests` — test: agent has prior belief about target's inventory (5 bread), target now has 2 bread, passive observation emits Discovery with InventoryDiscrepancy
3. `crates/worldwake-systems/src/perception.rs::tests` — test: agent has no prior belief, first observation does NOT emit Discovery
4. `crates/worldwake-systems/src/perception.rs::tests` — test: agent's prior belief matches current state, no Discovery emitted

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
