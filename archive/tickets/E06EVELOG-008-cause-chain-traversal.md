# E06EVELOG-008: Cause Chain Traversal and Reverse Effect Index

## Archive Amendment (2026-03-09)

The final authoritative `EventLog` implementation lives in `worldwake-core`, not `worldwake-sim`. This archived ticket captures the intermediate traversal plan before the event journal was consolidated beside `World`.

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends `EventLog` with traversal methods and reverse index
**Deps**: E06EVELOG-005 (EventLog with all indices exists)

## Problem

The spec requires traversable cause chains in both directions: tracing from an event back to its root cause, and looking up downstream effects of an event. Reverse effect lookup must be indexed (not a full scan). Causal depth measurement is needed for emergence metrics (spec 3.10).

## Assumption Reassessment (2026-03-09)

1. `EventLog` already exists from E06EVELOG-004/005 with append-only `events: Vec<EventRecord>`, `next_id`, and deterministic indices by tick/actor/place/tag — confirmed in `crates/worldwake-sim/src/event_log.rs`
2. `EventLog::emit(&mut self, pending: PendingEvent) -> EventId` is the only append surface and therefore the correct place to maintain any new causal index and enforce causal append invariants — confirmed
3. `PendingEvent` / `EventRecord` already carry `cause: CauseRef`, and `PendingEvent::new` already normalizes `target_ids` into stable sorted/deduped order — confirmed in `crates/worldwake-sim/src/event_record.rs`
4. `CauseRef::Event(EventId)` links an event to its direct cause, while `CauseRef::Bootstrap`, `SystemTick`, and `ExternalInput` are explicit roots — confirmed in `crates/worldwake-sim/src/cause.rs`
5. Existing `EventLog` tests already live inline in `crates/worldwake-sim/src/event_log.rs`; this ticket should extend that suite rather than introducing a parallel test module
6. `WorldTxn::commit` already funnels committed simulation events through `EventLog::emit`, so strengthening append-time validation improves the architecture without adding a second mutation path — confirmed in `crates/worldwake-sim/src/world_txn.rs`

## Architecture Check

1. The reverse effect index should mirror the existing index design: `by_cause: BTreeMap<EventId, Vec<EventId>>`, with `Vec<EventId>` preserving emission order for direct effects and `BTreeMap` preserving deterministic key ordering
2. `EventLog::emit` should enforce the causal invariant for `CauseRef::Event(cause_id)` at append time: the cause must already exist and must be strictly earlier than the new event. This is better than leaving invalid logs to be discovered later because `emit` is the single append boundary
3. `trace_cause_chain` should walk backward only through valid `CauseRef::Event(id)` links until hitting an explicit root cause (`Bootstrap`, `SystemTick`, `ExternalInput`)
4. `get_effects` should remain a direct index lookup that returns only one-hop effects; recursive effect-tree traversal is a separate concern
5. `causal_depth` should count hops from the event back to its explicit root cause
6. Traversal is deterministic because each event has exactly one direct cause and append-time validation prevents forward references or dangling cause links from entering the log

## What to Change

### 1. Add reverse cause index to `EventLog`

```rust
by_cause: BTreeMap<EventId, Vec<EventId>>,
```

### 2. Update `emit` to validate cause refs and maintain reverse index

When emitting a record:
- if `cause` is `CauseRef::Event(cause_id)`, require `cause_id < new_event_id`
- require `event_log.get(cause_id)` to exist before append
- push the new event ID into `by_cause[cause_id]`

This should be enforced as a hard append invariant, not merely a debug-only hint. Invalid causal references represent programmer error at the only authoritative append boundary.

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

### 4. Keep traversal small and trust the validated log

Do not build a second corruption-recovery layer into traversal. Once `emit` enforces backward-only, existing-cause links, cycles are structurally impossible in normal code. A small debug assertion inside traversal is acceptable, but the primary guarantee belongs at append time.

## Files to Touch

- `crates/worldwake-sim/src/event_log.rs` (modify — add `by_cause`, update `emit`, add traversal methods, extend inline tests)

## Out of Scope

- Recursive effect tree traversal (get_effects is single-hop by design)
- Cause chain visualization or formatting
- Completeness verification (E06EVELOG-009)
- WorldTxn changes (traversal is read-only on the log)
- Generalizing secondary indices behind a shared abstraction; the direct `BTreeMap<Key, Vec<EventId>>` pattern is still the cleanest fit here

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
9. Reverse effect lookup is deterministic and preserves emission order within a cause key
10. `by_cause` index survives bincode round-trip
11. Emitting an event with `CauseRef::Event` pointing to a missing event is rejected immediately
12. Emitting an event with `CauseRef::Event` pointing to itself or a future event is rejected immediately
13. Existing suite: `cargo test --workspace`

### Invariants

1. Every cause chain reaches an explicit root cause (spec 9.3: no orphan mutations)
2. Reverse effect lookup is indexed, not a full scan (spec requirement)
3. Traversal is deterministic (determinism invariant)
4. Cause references only point backward (cause.event_id < effect.event_id), and invalid references are rejected at append time

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/event_log.rs` — root cause chain (depth 0), linear chain traversal, branching effects (A causes B and C), empty direct effects, causal depth measurement, reverse-index round-trip, reject missing-cause append, reject self/future-cause append

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`

## Outcome

- Outcome amended: 2026-03-09
- Completion date: 2026-03-09
- What actually changed: extended `crates/worldwake-core/src/event_log.rs` with a deterministic `by_cause` reverse index, added `trace_cause_chain`, `get_effects`, and `causal_depth`, and strengthened `emit` so event-caused appends must reference an existing earlier event
- Deviations from original plan: corrected the ticket first so it matched the current `PendingEvent`/inline-test architecture, and tightened scope so causal validity is enforced at append time instead of being treated as a debug-only traversal concern; the final implementation also lives in `worldwake-core`, which is the cleaner long-term ownership boundary
- Verification results: `cargo test -p worldwake-core`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo fmt --check` passed
