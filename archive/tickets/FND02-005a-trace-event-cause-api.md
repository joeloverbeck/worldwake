# FND02-005a: Replace EventLog causal-trace API with ancestry-oriented trace_event_cause()

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — replace/reshape causal trace API in worldwake-core and update dependents
**Deps**: Phase 2 complete (event log already stores causal links and exposes traversal)

## Problem

The original ticket assumption was wrong. A structured causal inspection API already exists, but it is shaped around the CLI's backward trace display rather than around the programmatic question "what caused this event?" `EventLog::trace_cause_chain()` currently returns `[self, parent, root, ...]` in reverse chronological order and lives in `worldwake-core`, not `worldwake-sim`. For debuggability and long-term API clarity, the causal inspection API should be owned by the event-log type and should expose ancestry in causal order, not introduce a second traversal helper in a higher crate.

## Assumption Reassessment (2026-03-14)

1. `EventLog`, `EventRecord`, and `CauseRef` live in `worldwake-core`, not `worldwake-sim` — confirmed in `crates/worldwake-core/src/event_log.rs`, `event_record.rs`, and `cause.rs`.
2. `EventLog` already has `by_cause: BTreeMap<EventId, Vec<EventId>>` and `get_effects()` — confirmed.
3. `EventLog` already has `trace_cause_chain(event_id)` and `causal_depth(event_id)` — confirmed.
4. `trace_cause_chain()` currently returns the queried event first and walks backward to the root: `[self, parent, root, ...]` — confirmed by existing core tests.
5. `worldwake-cli` already depends on `trace_cause_chain()` for event-trace display — confirmed in `crates/worldwake-cli/src/handlers/events.rs` and CLI tests.
6. No `event_trace.rs` exists in `worldwake-sim`, but creating one would duplicate logic across crate boundaries and misplace ownership — corrected scope.

## Architecture Check

1. This remains a **derived read-model** — it traverses existing event log data without storing anything new.
2. The clean ownership boundary is `worldwake-core::EventLog`; adding a parallel sim-layer helper would create two traversal APIs over the same data and blur crate responsibilities.
3. No backwards-compatibility aliases. Replace the existing public traversal API with the better-shaped one and fix the call sites inside the workspace.
4. Reuse the existing append-only ordering invariant (`CauseRef::Event` always points to an earlier event) rather than introducing any new index or cache.

## Scope Correction

Do **not** create `crates/worldwake-sim/src/event_trace.rs`.

Do **not** add a free function that duplicates `EventLog` traversal in another crate.

Instead, replace the existing `EventLog` traversal entry point with the ancestry-oriented API the original ticket was trying to introduce.

## What to Change

### 1. Replace the causal traversal method on `EventLog`

```rust
/// Return the causal ancestry of `event_id`, ordered from the oldest
/// ancestor to the immediate parent.
///
/// Returns an empty Vec if the event has no event parent.
pub fn trace_event_cause(&self, event_id: EventId) -> Vec<EventId>
```

Implementation:
- Start from `event_id`.
- Walk `CauseRef::Event(parent_id)` links through the existing event log.
- Collect only ancestor event ids, not the queried event id itself.
- Return the result in causal order from oldest ancestor to immediate parent.
- Root causes (`Bootstrap`, `SystemTick`, `ExternalInput`) terminate the walk and yield no ancestor for that hop.
- Preserve defensive termination if an event lookup fails unexpectedly.

This replaces `trace_cause_chain()` rather than adding an alias.

### 2. Update dependent code to the new semantics

- Update `causal_depth()` to match the new ancestry shape directly.
- Update the CLI event trace handler to print the queried event first and then the returned ancestors in reverse for human-readable backward tracing.
- Update any tests that currently assert the old `[self, parent, root]` contract.

## Files to Touch

- `crates/worldwake-core/src/event_log.rs`
- `crates/worldwake-cli/src/handlers/events.rs`
- affected tests in core and CLI

## Out of Scope

- Do NOT implement `explain_goal()` — that is FND02-005b.
- Do NOT add a new traversal module to `worldwake-sim`.
- Do NOT modify `EventRecord` or `CauseRef` structures.
- Do NOT add new stored state, indices, or caches.
- Do NOT change the append-only event-log invariants.
- Do NOT add stored state — this must remain a derived read-model.

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: Create a chain of 3+ causally linked events (A causes B, B causes C). `trace_event_cause(C)` returns `[A, B]`.
2. Unit test: `trace_event_cause()` on an event with `CauseRef::Bootstrap` returns empty vec.
3. Unit test: `trace_event_cause()` on an event with `CauseRef::SystemTick` returns empty vec.
4. Unit test: `trace_event_cause()` on an event with `CauseRef::ExternalInput` returns empty vec.
5. Unit test: Single-parent chain (A causes B) returns `[A]`.
6. Existing CLI trace coverage is updated to the new API shape and still prints the same human-readable backward trace.
7. Relevant suites: `cargo test -p worldwake-core`, `cargo test -p worldwake-cli`
8. Full suite: `cargo test --workspace`

### Invariants

1. API remains a pure derived read-model — no state stored, no mutations.
2. No `HashMap`, `HashSet`, `f32`, `f64` in new code.
3. Append-only event log invariant preserved — traversal only reads existing records.
4. Deterministic — same event log + event ID always returns the same ancestry.
5. Ownership stays with `EventLog`; there is only one causal traversal API path.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_log.rs` — unit tests for ancestry traversal, terminal causes, and depth semantics.
2. `crates/worldwake-cli/src/handlers/events.rs` / CLI integration tests — update trace assertions to the new API shape without changing user-visible trace output.

### Commands

1. `cargo test -p worldwake-core event_log` — targeted core tests
2. `cargo test -p worldwake-cli trace` — targeted CLI trace tests
3. `cargo clippy --workspace` — lint check
4. `cargo test --workspace` — full workspace suite

## Outcome

- **Completion date**: 2026-03-14
- **What actually changed**: Replaced `EventLog::trace_cause_chain()` in `worldwake-core` with `EventLog::trace_event_cause()`, which returns ancestor event ids in causal order from oldest ancestor to immediate parent. Updated `causal_depth()` to match the new contract directly. Updated CLI trace code to consume the core API while preserving the same human-readable backward trace output.
- **Deviation from original plan**: The original ticket proposed a new `worldwake-sim` free function and module. That would have duplicated traversal logic above the owning crate. The implementation instead kept causal traversal owned by `worldwake-core::EventLog` and updated dependent CLI/tests accordingly.
- **Verification results**:
  - `cargo test -p worldwake-core event_log`
  - `cargo test -p worldwake-cli trace`
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-cli`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
