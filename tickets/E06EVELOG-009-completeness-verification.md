# E06EVELOG-009: Completeness Verification

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new verification function in `worldwake-sim`
**Deps**: E06EVELOG-008 (cause chain traversal exists), E06EVELOG-007 (WorldTxn commit exists)

## Problem

The spec requires a `verify_completeness(world, event_log)` function that mechanically proves every authoritative mutation path is journaled. This is the capstone of E06 — without it, "every persistent state change has exactly one cause" remains aspirational rather than enforceable.

## Assumption Reassessment (2026-03-09)

1. `EventLog` exists with all indices and traversal methods from E06EVELOG-004/005/008 — prerequisite
2. `WorldTxn` exists with commit flow from E06EVELOG-006/007 — prerequisite
3. `World` has component/relation/entity mutation methods — confirmed
4. `StateDelta` variants cover: Entity, Component, Relation, Quantity, Reservation — prerequisite (E06EVELOG-003)
5. E03/E05 narrowed the mutation surface so that all world writes go through known `World` methods — confirmed
6. The spec says completeness is enforceable because the mutation surface is now finite and known

## Architecture Check

1. `verify_completeness` checks two properties:
   - **Delta coverage**: every `EventRecord` in the log has at least the deltas that would result from its mutations (no phantom events with missing deltas)
   - **Orphan detection**: walk the event log and verify every event has a valid cause chain reaching a root cause
2. Full enforcement that ALL world mutations go through WorldTxn is a convention enforced by code review and API design, not runtime — `verify_completeness` catches violations when they produce observable inconsistencies
3. A test harness function `verify_no_out_of_band_mutation` demonstrates that deliberate bypass is caught: mutate world directly, then call verify — it should fail
4. This is primarily a test-time and debug-time tool, not a hot-path runtime check

## What to Change

### 1. Create `crates/worldwake-sim/src/verification.rs`

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

### 2. Implement verification checks

1. **Sequential ID check**: iterate events, verify IDs are 0, 1, 2, ... N-1
2. **Cause validity check**: for each event with `CauseRef::Event(cause_id)`, verify `cause_id < event_id` and that `event_log.get(cause_id)` exists
3. **Root reachability check**: for each event, verify `trace_cause_chain` terminates at an explicit root cause (not by running out of events)
4. Return all errors found (don't stop at first)

### 3. Add test-only bypass detection helper

```rust
#[cfg(test)]
pub fn verify_event_covers_world_state(
    world: &World,
    event_log: &EventLog,
) -> Result<(), Vec<VerificationError>>
```

This is a stronger check used only in tests: verify that the current world state is consistent with the cumulative deltas in the event log. Initially this can check entity count consistency and commodity totals.

### 4. Register module in `crates/worldwake-sim/src/lib.rs`

## Files to Touch

- `crates/worldwake-sim/src/verification.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — register module, re-export)

## Out of Scope

- Runtime enforcement of WorldTxn-only mutation (convention, not runtime check)
- Full world-state reconstruction from event log (replay — E08)
- Performance optimization of verification (it's a debug/test tool)
- Cross-crate mutation prevention via visibility (Rust module system handles this)

## Acceptance Criteria

### Tests That Must Pass

1. `verify_completeness` passes on an empty event log
2. `verify_completeness` passes on a well-formed log with root causes and valid chains
3. `verify_completeness` catches a dangling cause reference (cause points to non-existent event)
4. `verify_completeness` catches a non-monotonic event ID
5. `verify_completeness` catches a gap in event ID sequence
6. `verify_completeness` catches an orphan event whose cause chain doesn't reach a root
7. `verify_completeness` reports ALL errors, not just the first one
8. Test harness: create events via WorldTxn → verify passes (spec T07)
9. Test harness: deliberately bypass WorldTxn to mutate world → show that the verification can detect the inconsistency
10. Existing suite: `cargo test --workspace`

### Invariants

1. Every event's cause chain reaches an explicit root cause (spec 9.3)
2. Event IDs are sequential and gapless (spec requirement)
3. Cause references only point to earlier events (causality flows forward)
4. Verification is deterministic (determinism invariant)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/verification.rs` — empty log, valid log, dangling cause, non-monotonic ID, ID gap, orphan chain, multi-error reporting, WorldTxn integration (end-to-end: mutations via txn → verify passes), out-of-band mutation detection

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-sim verification`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
5. `cargo fmt --check`
