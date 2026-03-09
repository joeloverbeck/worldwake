# E06EVELOG-008: Cause Chain Traversal and Reverse Effect Index

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends `EventLog` with traversal methods and reverse index
**Deps**: E06EVELOG-005 (EventLog with all indices exists)

## Problem

The spec requires traversable cause chains in both directions: tracing from an event back to its root cause, and looking up downstream effects of an event. Reverse effect lookup must be indexed (not a full scan). Causal depth measurement is needed for emergence metrics (spec 3.10).

## Assumption Reassessment (2026-03-09)

1. `EventLog` exists from E06EVELOG-004/005 with `events`, `next_id`, and indices by tick/actor/place/tag — prerequisite
2. `EventRecord` has `cause: CauseRef` — prerequisite (E06EVELOG-003)
3. `CauseRef::Event(EventId)` links an event to its direct cause — prerequisite (E06EVELOG-001)
4. `CauseRef::Bootstrap`, `SystemTick`, `ExternalInput` are explicit root causes — prerequisite (E06EVELOG-001)

## Architecture Check

1. The reverse effect index (`by_cause: BTreeMap<EventId, Vec<EventId>>`) maps a cause event to its direct effects — maintained by `emit`
2. `trace_cause_chain` walks backward through `CauseRef::Event(id)` links until hitting a root cause (`Bootstrap`, `SystemTick`, `ExternalInput`)
3. `get_effects` is a single index lookup, not recursion — returns only direct effects
4. `causal_depth` counts hops from the event back to its root cause
5. Traversal is deterministic because it follows a linear chain (each event has exactly one cause)

## What to Change

### 1. Add reverse cause index to `EventLog`

```rust
by_cause: BTreeMap<EventId, Vec<EventId>>,
```

### 2. Update `emit` to maintain reverse index

When emitting a record with `cause: CauseRef::Event(cause_id)`, push the new event's ID into `by_cause[cause_id]`.

### 3. Add traversal methods

```rust
/// Walk backward from event_id to root cause, returning the full chain
/// (starting with event_id, ending with the root).
pub fn trace_cause_chain(&self, event_id: EventId) -> Vec<EventId>
```

```rust
/// Return direct downstream effects of an event (one-hop only).
pub fn get_effects(&self, event_id: EventId) -> &[EventId]
```

```rust
/// Count hops from event back to its root cause.
pub fn causal_depth(&self, event_id: EventId) -> u32
```

### 4. Add cycle detection guard

`trace_cause_chain` should include a depth limit or visited-set guard to prevent infinite loops in case of data corruption. Since event IDs are monotonic and causes must reference earlier events, cycles should be structurally impossible — but a debug assertion is cheap insurance.

## Files to Touch

- `crates/worldwake-sim/src/event_log.rs` (modify — add by_cause index, update emit, add traversal methods)

## Out of Scope

- Recursive effect tree traversal (get_effects is single-hop by design)
- Cause chain visualization or formatting
- Completeness verification (E06EVELOG-009)
- WorldTxn changes (traversal is read-only on the log)

## Acceptance Criteria

### Tests That Must Pass

1. `trace_cause_chain` returns `[event_id]` for events with root causes (Bootstrap, SystemTick, ExternalInput)
2. `trace_cause_chain` returns full chain for a 3-deep causal chain: `[C, B, A]` where C caused by B caused by A (root)
3. `trace_cause_chain` terminates at explicit root causes, not at `None`
4. `get_effects` returns empty slice for events with no downstream effects
5. `get_effects` returns correct direct effects for a cause event
6. `get_effects` does NOT return indirect effects (grandchildren)
7. `causal_depth` returns 0 for root-cause events
8. `causal_depth` returns correct depth for multi-hop chains
9. Reverse effect lookup is deterministic (same results regardless of query order)
10. `by_cause` index survives bincode round-trip
11. Monotonic ID enforcement means cause IDs are always less than effect IDs (debug assertion)
12. Existing suite: `cargo test --workspace`

### Invariants

1. Every cause chain reaches an explicit root cause (spec 9.3: no orphan mutations)
2. Reverse effect lookup is indexed, not a full scan (spec requirement)
3. Traversal is deterministic (determinism invariant)
4. Cause references only point backward (cause.event_id < effect.event_id)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/event_log.rs` — root cause chain (depth 0), linear chain traversal, branching effects (A causes B and C), causal depth measurement, empty effects, reverse index round-trip, cause-must-precede-effect assertion

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`
