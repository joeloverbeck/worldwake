# E06EVELOG-009: Completeness Verification

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — event journal boundary moved into `worldwake-core`, with new verification and transaction modules there
**Deps**: E06EVELOG-008 (cause chain traversal exists), E06EVELOG-007 (WorldTxn commit exists)

## Problem

The spec requires a completeness check, and the clean architecture for that is to put the authoritative mutation boundary next to `World` itself. The earlier intermediate design kept journaling in `worldwake-sim`, which prevented `worldwake-core` from actually sealing the write surface.

This ticket therefore expanded from “verification only” into the architectural correction needed to make completeness enforceable in the right place: move the journal/event stack into `worldwake-core`, close direct `World` mutation APIs to crate visibility, extend `WorldTxn` to cover the remaining mutation families, and keep verification alongside the authoritative state model.

## Assumption Reassessment (2026-03-09)

1. `EventLog` exists with all indices and traversal methods from E06EVELOG-004/005/008 — prerequisite
2. `WorldTxn` exists with commit flow from E06EVELOG-006/007 — prerequisite
3. `World` has component/relation/entity mutation methods — confirmed
4. `StateDelta` variants cover: Entity, Component, Relation, Quantity, Reservation — prerequisite (E06EVELOG-003)
5. The intermediate implementation had journaling in `worldwake-sim`, but `World` itself lives in `worldwake-core`; that crate split prevented the mutation boundary from being cleanly enforced
6. `WorldTxn` did not yet cover the full `World` mutation surface (ownership/social mutations and lot split/merge were still direct world operations)
7. Because of 5-6, full causal completeness was **not** yet mechanically enforceable; fixing that boundary was more beneficial than preserving the intermediate layering

## Architecture Check

1. The clean boundary is `World` + `WorldTxn` + event journal inside `worldwake-core`, not split across crates
2. `verify_completeness` checks structural log integrity: event IDs are sequential and gapless, event causes are valid and backward-pointing, and every cause chain reaches an explicit root
3. The test-only audit helper reconstructs the journaled world surface from `StateDelta`s and compares it to the real `World`
4. The public write API should be `WorldTxn`; direct `World` mutators should not remain externally callable
5. This remains primarily a debug/test verification tool, not a hot-path runtime check

## What to Change

### 1. Move the journal boundary into `worldwake-core`

Create or relocate:
- `crates/worldwake-core/src/cause.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/event_log.rs`
- `crates/worldwake-core/src/event_record.rs`
- `crates/worldwake-core/src/event_tag.rs`
- `crates/worldwake-core/src/visibility.rs`
- `crates/worldwake-core/src/witness.rs`
- `crates/worldwake-core/src/world_txn.rs`
- `crates/worldwake-core/src/verification.rs`

Update `worldwake-core` exports so the authoritative event/journal API lives there.

### 2. Close the direct `World` write surface

Make authoritative mutation methods crate-visible and route external persistent writes through `WorldTxn`.

Required mutation families for this ticket:
- entity creation/archive
- placement
- reservations
- ownership
- social relations and knowledge relations
- lot split/merge quantity changes

### 3. Implement verification

Define:
```rust
pub fn verify_completeness(
    event_log: &EventLog,
) -> Result<(), Vec<VerificationError>>
```

`VerificationError` enum:
- `OrphanEvent { event_id: EventId }` — event's cause chain doesn't reach a root
- `DanglingCauseRef { event_id: EventId, cause: CauseRef }` — cause references a non-existent event
- `NonMonotonicId { event_id: EventId, expected: EventId }` — event ID out of sequence
- `GapInSequence { expected: EventId, found: EventId }` — gap in event ID sequence
- `FutureCauseRef { event_id: EventId, cause: CauseRef }` — cause points to the same or a later event
- `WorldStateMismatch { detail: String }` — test-only world/journal audit found live state that the journal does not explain

Implement verification checks:

1. **Sequential ID check**: iterate events, verify IDs are 0, 1, 2, ... N-1
2. **Cause validity check**: for each event with `CauseRef::Event(cause_id)`, verify `cause_id < event_id` and that the referenced event exists
3. **Root reachability check**: for each event, verify `trace_cause_chain` terminates at an explicit root cause (not by running out of events)
4. Return all errors found (don't stop at first)

### 4. Add test-only bypass detection helper

```rust
#[cfg(test)]
pub fn verify_event_covers_world_state(
    world: &World,
    event_log: &EventLog,
) -> Result<(), Vec<VerificationError>>
```

This is a stronger check used only in tests: reconstruct the currently journaled world surface from cumulative deltas and compare it to the real `World`.

Required audit surface for this ticket:
- live entities and archive state
- authoritative component values represented in `ComponentDelta`
- authoritative relations represented in `RelationDelta`
- live reservations represented in `ReservationDelta`

Explicitly out of scope for this ticket:
- mutation families that are not yet journaled by `WorldTxn`
- proving that no unjournaled `World` API exists
- replaying arbitrary world state from the log for production use

### 5. Remove obsolete `worldwake-sim` ownership of the event/journal API

No backwards-compatibility re-export path. `worldwake-sim` should stop owning or aliasing the authoritative mutation boundary.

## Files to Touch

- `crates/worldwake-core/src/lib.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/world/*.rs`
- `crates/worldwake-core/src/world_txn.rs`
- `crates/worldwake-core/src/verification.rs`
- `crates/worldwake-core/tests/relation_invariants.rs`
- `crates/worldwake-sim/src/lib.rs`
- relevant `Cargo.toml` files

## Out of Scope

- Full world-state reconstruction from event log (replay — E08)
- Performance optimization of verification (it's a debug/test tool)

## Acceptance Criteria

### Tests That Must Pass

1. `verify_completeness` passes on an empty event log
2. `verify_completeness` passes on a well-formed log with root causes and valid chains
3. `verify_completeness` catches a dangling cause reference (cause points to non-existent event)
4. `verify_completeness` catches a non-monotonic event ID
5. `verify_completeness` catches a gap in event ID sequence
6. `verify_completeness` catches an orphan event whose cause chain doesn't reach a root
7. `verify_completeness` reports ALL errors, not just the first one
8. Test harness: create events via `WorldTxn` in `worldwake-core` → verify passes (spec T07)
9. Test harness: deliberately bypass `WorldTxn` on an audited mutation surface from inside crate tests → show that the world/journal audit detects the inconsistency
10. Existing suite: `cargo test --workspace`
11. Relation invariant integration tests use the new transaction boundary instead of direct public `World` writes

### Invariants

1. Every event's cause chain reaches an explicit root cause (spec 9.3)
2. Event IDs are sequential and gapless (spec requirement)
3. Cause references only point to earlier events (causality flows forward)
4. Verification is deterministic (determinism invariant)
5. External persistent writes go through `WorldTxn`
6. Test-only world/journal audit only claims coverage for the surface that is represented by committed deltas

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/verification.rs` — empty log, valid log, dangling cause, non-monotonic ID, ID gap, orphan chain, multi-error reporting, `WorldTxn` integration, out-of-band mutation detection
2. `crates/worldwake-core/src/world_txn.rs` — ownership/social/quantity wrapper delta coverage
3. `crates/worldwake-core/tests/relation_invariants.rs` — invariant tests migrated to the transaction boundary

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo fmt --check`

## Outcome

Implemented:
- moved the event journal, transaction layer, and verification modules from `worldwake-sim` into `worldwake-core`, so the mutation boundary now sits next to `World`
- closed direct authoritative `World` write APIs to crate visibility and made `WorldTxn` the external persistent-write boundary
- extended `WorldTxn` to cover ownership, social, knowledge, and lot split/merge mutation families in addition to entity, placement, and reservation mutations
- implemented `verify_completeness(&EventLog)` with deterministic multi-error reporting for gapless IDs, non-monotonic IDs, dangling causes, forward/self causes, and orphaned cause chains
- implemented a test-only `verify_event_covers_world_state(&World, &EventLog)` audit that reconstructs the journaled surface from deltas and compares it to live world state
- migrated relation invariant integration tests to the transaction boundary and added delta-coverage tests for the new wrappers

Changed vs originally planned:
- went further than the intermediate verification-only plan because the split between `worldwake-core` and `worldwake-sim` was the wrong architecture for enforcing journal completeness
- removed the backwards-compatibility alias path instead of preserving it; the authoritative event/journal API now lives only in `worldwake-core`
- still kept the world/journal audit honest to the journaled surface represented by committed deltas rather than pretending to provide a general replay engine
