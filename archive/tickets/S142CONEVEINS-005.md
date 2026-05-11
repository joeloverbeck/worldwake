# S142CONEVEINS-005: Populate `BlockingFact::ReservationConflict.contention_event` from event-log lookup

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small-Medium
**Engine Changes**: Yes — `worldwake-ai` agent_tick execution path looks up contention events
**Deps**: archive/tickets/S142CONEVEINS-001.md (provides `EventTag::ContentionResolved`), archive/tickets/S142CONEVEINS-002.md (provides `BlockingFact::ReservationConflict.contention_event` field), archive/tickets/S142CONEVEINS-003.md (facility-queue emission), archive/tickets/S142CONEVEINS-004.md (resource-extraction emission)

## Problem

After ticket 002 lands, `BlockingFact::ReservationConflict` carries `contention_event: Option<EventId>` defaulting to `None` at all construction sites. After tickets 003 and 004 land, `ContentionResolved` events are present in the event log at every grant. But the AI never connects the two: the construction sites in `agent_tick/execution.rs` and `failure_handling.rs` do not look up the resolving event by `(affordance, at_tick)` and populate the field. This ticket completes the end-to-end attribution: when a reservation-conflict failure is recorded for an agent at tick T against an affordance, the AI looks up `events_by_tag(EventTag::ContentionResolved)` filtered by `(affordance, at_tick == T)`, and if a matching event exists, sets `contention_event = Some(event_id)`.

## Assumption Reassessment (2026-05-10)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-ai/src/agent_tick/execution.rs` exists and is the spec's named target for the lookup. Construction sites for `BlockingFact::ReservationConflict` are concentrated in `failure_handling.rs` (10 runtime sites: `:420`, `:525`, `:541`, `:736`, `:771`, `:838`, `:959`, `:1234`, `:1250`, `:1450`). Whether the lookup is performed at the `agent_tick/execution.rs` decision boundary (centralizing population) OR at each `failure_handling.rs` construction site (distributing population) is an implementation question to resolve during reassessment. Centralizing is preferred: post-`failure_handling.rs` returns to `agent_tick/execution.rs`, which observes the produced `BlockingFact` and can mutate `contention_event` before persisting.
2. `EventLog::events_by_tag` is at `crates/worldwake-core/src/event_log.rs:124` with signature `pub fn events_by_tag(&self, tag: EventTag) -> &[EventId]`. The lookup must walk the slice and resolve each `EventId` through the event log to inspect its payload (filter by `affordance` and `at_tick`). For the headline case (one resolution per `(affordance, tick)` pair), there is at most one match. Multiple matches at the same tick on the same affordance would indicate a bug — assert single match in the focused test.
3. The shared abstraction boundary under audit is the `BlockingFact::ReservationConflict` populated/unpopulated state. Per `docs/precision-rules.md` Rule 16 (information-path refactors), the canonical post-implementation transport is: contention event written by tickets 003/004 → looked up via `events_by_tag` → populated into `BlockingFact::ReservationConflict.contention_event` → consumed by decision trace and downstream AI. No alias path: `contention_event` is the only carrier of this attribution.
4. Per Rule 6 (decision-trace preference): the populated `contention_event` is the strongest available lower-layer proof for AI reasoning about reservation conflicts. The decision trace surfaces the populated field; future debugging tools can resolve the `EventId` to inspect the resolution.
5. Lookup cost: the spec's Risks section flags discrepancy backreference latency. For this ticket's scope, the naive linear walk over `events_by_tag(EventTag::ContentionResolved)[..]` is acceptable. If profiling shows cost growth, a per-affordance index can land in a follow-up ticket per the spec's Risks mitigation note.

## Architecture Check

1. The lookup is read-only; no game-state mutation occurs. Per FND-14 and FND-14A, the AI consults event-log history (authoritative causal record), not world state directly. The event log is queryable per FND-29A.
2. Per FND-26, the AI reads through the canonical `events_by_tag` accessor; no direct call into `worldwake-systems` to retrieve resolution data.
3. Centralizing the lookup at the `agent_tick/execution.rs` boundary keeps the population logic in one place. `failure_handling.rs` continues to construct `BlockingFact::ReservationConflict` with `contention_event: None` (defaulting from ticket 002), and `agent_tick/execution.rs` performs the post-construction lookup-and-populate on the resulting `BlockingFact` before persistence. This matches the spec's text: "The AI's `agent_tick/execution.rs` populates `contention_event` when the conflict path observes a `ContentionResolved` event for the same affordance at the conflict tick."
4. Per FND-28, no fallback-to-`None`-with-shim path coexists with the populated path. If lookup finds no match, `contention_event` stays `None` — that is the legitimate "no resolution event was emitted at this tick for this affordance" state, not a shim.

## Verification Layers

1. Lookup behavior — focused unit/runtime test in `agent_tick/execution.rs` test module: a `BlockingFact::ReservationConflict` with a contention event present in the log gets `contention_event: Some(_)` populated; with no event present, `contention_event` stays `None`.
2. Decision trace surfaces the populated field because `BlockingFact` is carried as-is in the existing `BlockerRecorded` payload; direct trace-sink/end-to-end assertion remains ticket 007's golden 4.
3. End-to-end attribution chain (failure → look up event → populate field → trace exposes it) is covered by ticket 007's golden 4 (`BlockingFact::ReservationConflict` with non-`None` `contention_event`)
4. Per Rule 5 (verification surface mapping): this ticket's contract is the population behavior. Lower-layer (event-log writes) is verified by tickets 003/004; upper-layer (end-to-end) is verified by ticket 007. This ticket's proof is the focused unit/runtime test on the lookup function itself.

## What to Change

### 1. Add a lookup-and-populate helper in `agent_tick/execution.rs`

Add `populate_contention_event_refs`, which takes a `BlockerMemory`, the agent's tick, and the event log; for each `BlockingFact::ReservationConflict { affordance, contention_event: None }`, walk `event_log.events_by_tag(EventTag::ContentionResolved)`, resolve each event's payload, and populate the copied blocker memory with `contention_event = Some(event_id)` when the payload's `contested_affordance == affordance` and `at_tick == tick`. If multiple matches exist (bug condition), prefer the first match and surface a debug assertion.

### 2. Call the helper at the `BlockingFact` finalization point in `agent_tick/execution.rs`

Identify the boundary where `BlockingFact` from `failure_handling.rs` is consumed by the decision trace or persisted into the agent's blocker memory. Insert the call to the helper before that point. The lookup runs once per blocker-recording event — typically O(N) where N is the number of `ContentionResolved` events at the current tick (small in practice).

### 3. Focused unit/runtime test

In the `#[cfg(test)]` block of `agent_tick/execution.rs` (or a sibling test module), add a focused test:
- Construct a fixture event log with one `ContentionResolved` event at tick T against affordance A.
- Construct a `BlockingFact::ReservationConflict { affordance: A, contention_event: None }`.
- Call the helper.
- Assert `contention_event == Some(event_id)`.

Plus a negative test:
- Same fixture but the event has affordance B (not A).
- Assert `contention_event == None` after the call.

Plus a tick-mismatch test:
- Event at tick T-1, blocker at tick T.
- Assert `contention_event == None`.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify)

## Out of Scope

- Construction-side population in `failure_handling.rs` — explicit non-goal; lookup is centralized in `agent_tick/execution.rs`
- Per-affordance lookup index for performance optimization — flagged in spec Risks; deferred to a follow-up ticket if profiling warrants
- Decision trace schema changes for the contention-event field — the existing decision-trace serialization carries `BlockingFact` as-is; the populated field is automatically visible
- Observer rendering of `contention_event` (ticket 006)
- Goldens (ticket 007)

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: matching event populates `contention_event`.
2. New focused test: non-matching affordance leaves `contention_event` None.
3. New focused test: tick mismatch leaves `contention_event` None.
4. Existing suite: `cargo test -p worldwake-ai agent_tick`.

### Invariants

1. The lookup is read-only; no event-log mutation occurs.
2. When a `ContentionResolved` event matches `(affordance, tick)`, `contention_event` is populated; otherwise it stays `None`.
3. The lookup walks `events_by_tag(EventTag::ContentionResolved)` only — no other event tags or world-state queries.
4. Per FND-14, the lookup does not consult world state for ground truth; it uses the authoritative event log.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/execution.rs` (extended `#[cfg(test)]` block) — 3 focused tests covering positive lookup, affordance mismatch, tick mismatch.

### Commands

1. `cargo test -p worldwake-ai --lib populate_contention_event_ref`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-11.

- Added `populate_contention_event_refs` in `crates/worldwake-ai/src/agent_tick/execution.rs`, called from `finalize_agent_tick` before `persist_blocked_memory`, so newly recorded `BlockingFact::ReservationConflict` entries are lazily enriched from `events_by_tag(EventTag::ContentionResolved)` before the blocker memory component and `BlockerRecorded` decision payload are written.
- Added focused unit coverage for matching `(affordance, at_tick)` population, affordance mismatch, and tick mismatch. The lookup remains read-only and leaves `contention_event` as `None` when no matching event exists.

## Deviations

- The landed helper operates over `BlockerMemory` lazily rather than mutating a standalone `BlockingFact`; this keeps the finalization boundary centralized and ensures the persisted component and decision trace payload use the same populated blocker value.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib populate_contention_event_ref -- --list` (selector resolved to the 3 focused `agent_tick::execution::tests::*` tests).
- Passed `cargo test -p worldwake-ai --lib populate_contention_event_ref`.
- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-ai agent_tick`.
- Passed `cargo test --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
