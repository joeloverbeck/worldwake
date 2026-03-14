# E15RUMWITDIS-013: Event-Local Perception Snapshots For Event-Based Belief Updates

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — event-record payload shape, world transaction capture, perception event-path updates
**Deps**: `archive/tickets/completed/E14PERBEL-005.md`, `archive/tickets/completed/E15RUMWITDIS-001.md`, `archive/tickets/completed/E15RUMWITDIS-009.md`, `specs/E15-rumor-witness-discovery.md`

## Problem

Event-based perception currently treats an event as a trigger, then rebuilds observed entity snapshots from the authoritative world state when `perception_system()` runs later in the tick. That collapses multiple same-tick mutations into one end-of-tick view and can make a witness "perceive" a later state that was not actually observable at the time of the event.

This weakens the architecture in three ways:

1. Discovery mismatches can be attributed to the wrong event-local state.
2. Multiple events touching the same entity in one tick lose causal distinctness in witness belief updates.
3. Perception correctness depends on reading the mutable world after the fact instead of relying on the append-only causal record itself.

## Assumption Reassessment (2026-03-14)

1. `archive/tickets/completed/E14PERBEL-005.md` explicitly chose the current architecture: event-driven belief refresh uses the event record as a trigger, then snapshots entities from authoritative post-event world state rather than reconstructing from raw deltas.
2. `archive/tickets/completed/E15RUMWITDIS-009.md` preserved that architecture and called out the remaining limitation: event-based mismatch detection still compares against post-event authoritative snapshots, not event-local observed state.
3. No active ticket in `tickets/` currently owns this limitation. `E15RUMWITDIS-010` is confidence derivation, `E15RUMWITDIS-011` is integration coverage, and `E15RUMWITDIS-012` is required-profile cleanup.
4. `EventRecord` already stores `state_deltas`, `target_ids`, `evidence`, `visibility`, and witness data, but it does not carry an event-local projection of what witnesses should learn about each observed entity.
5. Reconstructing witness snapshots later from arbitrary `StateDelta` batches would be brittle as `BelievedEntityState` evolves. A dedicated event-local observed-state payload is cleaner than adding more delta decoders to perception.

## Architecture Check

1. The clean long-term architecture is to make event-local observable state part of the append-only event record itself. Perception should consume the causal record, not reinterpret mutable end-of-tick world state after the fact.
2. This is more robust than decoding raw `state_deltas` inside `perception.rs`. `BelievedEntityState` has already grown beyond pure relation deltas, and future fields would keep making ad hoc delta reconstruction more fragile.
3. The event record should not store observer-specific belief state. Instead it should store a reusable event-local entity snapshot payload, and perception should stamp `observed_tick` and `PerceptionSource` when writing beliefs.
4. No backwards-compatibility shim: once event-local snapshots exist, event-based perception should stop falling back to end-of-tick authoritative rebuilds for entities covered by the event payload.

## What to Change

### 1. Add an event-local observed entity snapshot payload

Introduce a new core value type for the event log, for example:

```rust
pub struct ObservedEntitySnapshot {
    pub last_known_place: Option<EntityId>,
    pub last_known_inventory: BTreeMap<CommodityKind, Quantity>,
    pub workstation_tag: Option<WorkstationTag>,
    pub resource_source: Option<ResourceSource>,
    pub alive: bool,
    pub wounds: Vec<Wound>,
}
```

Then add an ordered map on `PendingEvent` / `EventRecord`, for example:

```rust
pub observed_entities: BTreeMap<EntityId, ObservedEntitySnapshot>
```

Requirements:

1. This payload represents event-local observable state, not belief state.
2. It must be deterministic and serializable.
3. It must stay aligned with the fields perception actually writes into `BelievedEntityState`.

### 2. Capture snapshots at event construction time

Add a helper that builds `ObservedEntitySnapshot` from the authoritative world at the moment an event is committed.

Recommended shape:

1. Reuse the shared belief projection ownership introduced by `E15RUMWITDIS-001`, but split observer-agnostic data from belief-specific metadata if needed.
2. `WorldTxn` builder methods should populate `observed_entities` for the actor, targets, and any other entities that the event intentionally exposes to witnesses.
3. Do not attempt to infer this later from `state_deltas` in perception.

This is the key architectural improvement: the event record becomes the stable carrier of what was observable when the event happened.

### 3. Switch event-based perception to event-local snapshots

Update `crates/worldwake-systems/src/perception.rs` so the event path:

1. reads `record.observed_entities`
2. converts each `ObservedEntitySnapshot` into the belief-layer `BelievedEntityState` by stamping:
   - `observed_tick = record.tick`
   - `source = PerceptionSource::DirectObservation`
3. runs the shared mismatch comparison against that event-local snapshot
4. updates the witness belief store from that event-local snapshot

If an event intentionally exposes no entity snapshots, it should simply produce no entity belief update for that path.

### 4. Add regression coverage for same-tick multi-event fidelity

Add tests proving the architecture fixes the real gap:

1. Two events in the same tick mutate the same entity in different ways.
2. Event-based witnesses receive the correct per-event snapshots in sequence.
3. Discovery mismatches are derived from the event-local snapshot attached to each event, not from the final world state after later same-tick mutations.

## Files to Touch

- `crates/worldwake-core/src/event_record.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify — exports if needed)
- `crates/worldwake-core/src/belief.rs` (modify only if a shared observer-agnostic projection type/helper belongs here)
- `crates/worldwake-core/src/world_txn.rs` (modify — populate event-local observed snapshots)
- `crates/worldwake-systems/src/perception.rs` (modify — consume event-local snapshots)
- `crates/worldwake-systems/src/*` action/system files that emit events and need explicit observed-entity registration (modify only where necessary)

## Out of Scope

- Passive same-place perception changes
- Tell action behavior or rumor propagation rules
- Crime interpretation, guard response, or accusation logic in E17
- Planner confidence derivation (`E15RUMWITDIS-010`)
- A generic full-world replay/reconstruction engine from raw `state_deltas`

## Acceptance Criteria

### Tests That Must Pass

1. Event-based perception updates beliefs from `EventRecord` event-local snapshots rather than rebuilding from end-of-tick authoritative world state
2. Two same-tick events touching the same entity can yield two distinct witness belief updates when their event-local snapshots differ
3. Event-based `PlaceChanged` discovery reflects the event-local snapshot carried by the event, not a later same-tick mutation
4. Existing passive perception behavior remains unchanged
5. Existing suite: `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Event-local observable state is carried by the append-only event record, not reconstructed later from mutable end-of-tick world state
2. Event records store observer-agnostic observable snapshots, never observer-specific belief metadata
3. Event-based perception uses one shared mismatch pipeline regardless of whether the snapshot source is passive observation or event-local observation
4. No fallback compatibility path remains where event-based perception silently ignores event-local snapshots and rebuilds from end-of-tick authoritative state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_record.rs` — add serialization and ordering coverage for the new observed-entity snapshot payload
2. `crates/worldwake-core/src/world_txn.rs` — add tests proving committed events capture event-local observed snapshots deterministically
3. `crates/worldwake-systems/src/perception.rs` — add same-tick multi-event witness tests proving event-local snapshots drive belief updates and mismatch detection
4. `crates/worldwake-systems/tests/` or existing integration suites — add one integration test covering sequential same-tick event observations on a shared subject

### Commands

1. `cargo test -p worldwake-core event_record`
2. `cargo test -p worldwake-systems perception`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`


---
