# FND02-005a: Add trace_event_cause() Debuggability API to Sim Crate

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new module in worldwake-sim
**Deps**: Phase 2 complete (EventLog with by_cause index exists)

## Problem

No structured causal inspection API exists for the event log. The simulation produces emergent behavior but provides no programmatic way to answer "what caused this event?" The event log already stores causal links (`by_cause: BTreeMap<EventId, Vec<EventId>>` in `event_log.rs` line ~16) and `CauseRef` chains, but no public API walks these chains. This is a core debuggability requirement per Principle 27 (Debuggability Is a Product Feature).

## Assumption Reassessment (2026-03-13)

1. `EventLog` has `by_cause: BTreeMap<EventId, Vec<EventId>>` — confirmed (line ~16 of `event_log.rs`).
2. `EventLog` has `get_effects()` for reverse lookup — confirmed.
3. `CauseRef` enum has `Event(EventId)`, `SystemTick(Tick)`, `Bootstrap`, `ExternalInput(u64)` — confirmed.
4. `EventRecord` stores `cause: CauseRef` — confirmed via `event_record.rs`.
5. No `event_trace.rs` exists in worldwake-sim — confirmed, must be created.
6. `worldwake-sim/src/lib.rs` lists all current public modules — confirmed, needs new module addition.

## Architecture Check

1. This is a **derived read-model** (Principle 25) — it traverses existing event log data without storing anything new. Pure computation over immutable log.
2. No backwards-compatibility shims — new API addition with no existing behavior changes.
3. Leverages existing `by_cause` index and `CauseRef` chain — no new data structures needed.

## What to Change

### 1. Create `crates/worldwake-sim/src/event_trace.rs`

```rust
/// Walk the CauseRef chain from the given event backwards through the
/// event log, returning the ordered causal ancestry (oldest cause first).
///
/// Returns an empty Vec if the event has no causal parent (Bootstrap or
/// ExternalInput causes terminate the chain).
pub fn trace_event_cause(
    event_log: &EventLog,
    event_id: EventId,
) -> Vec<EventId>
```

Implementation:
- Start from `event_id`.
- Look up the `EventRecord` via `event_log.get(event_id)`.
- If `cause` is `CauseRef::Event(parent_id)`, prepend `parent_id` and recurse/iterate.
- If `cause` is `SystemTick`, `Bootstrap`, or `ExternalInput`, stop.
- Return the chain in order from oldest ancestor to immediate parent.
- Guard against cycles (should not exist in append-only log, but defensive).

### 2. Wire into `crates/worldwake-sim/src/lib.rs`

Add `pub mod event_trace;` to the module declarations.

## Files to Touch

- `crates/worldwake-sim/src/event_trace.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module declaration)

## Out of Scope

- Do NOT implement `explain_goal()` — that is FND02-005b.
- Do NOT modify `EventLog`, `EventRecord`, or `CauseRef` structures.
- Do NOT add CLI integration or display formatting.
- Do NOT modify worldwake-core crate.
- Do NOT add stored state — this must remain a derived read-model.

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: Create a chain of 3+ causally linked events (A causes B, B causes C). `trace_event_cause(log, C)` returns `[A, B]`.
2. Unit test: `trace_event_cause()` on an event with `CauseRef::Bootstrap` returns empty vec.
3. Unit test: `trace_event_cause()` on an event with `CauseRef::SystemTick` returns empty vec.
4. Unit test: `trace_event_cause()` on an event with `CauseRef::ExternalInput` returns empty vec.
5. Unit test: Single-parent chain (A causes B) returns `[A]`.
6. Existing suite: `cargo test -p worldwake-sim`
7. Full suite: `cargo test --workspace`

### Invariants

1. Function is a pure derived read-model — no state stored, no mutations.
2. No `HashMap`, `HashSet`, `f32`, `f64` in new code.
3. Append-only event log invariant preserved — function only reads.
4. Deterministic — same event log + event ID always returns same ancestry.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/event_trace.rs` (inline test module) — unit tests for causal chain traversal, terminal causes, edge cases.

### Commands

1. `cargo test -p worldwake-sim -- event_trace` — targeted tests
2. `cargo test -p worldwake-sim` — full sim crate suite
3. `cargo clippy --workspace` — lint check
4. `cargo test --workspace` — full workspace suite
