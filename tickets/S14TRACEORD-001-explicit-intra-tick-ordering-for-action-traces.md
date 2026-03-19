# S14TRACEORD-001: Explicit Intra-Tick Ordering For Action Traces

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` action trace model and trace emission path, plus focused/unit coverage and downstream trace consumers where required
**Deps**: `archive/tickets/completed/S14CONMEME-001-same-place-office-fact-still-requires-tell.md`, `docs/FOUNDATIONS.md`, `AGENTS.md`, `docs/golden-e2e-testing.md`

## Problem

The current action trace system records `tick`, actor, action id/name, and lifecycle kind, but it does not expose first-class intra-tick ordering. For same-tick cross-agent chains, tests and debugging currently have to infer ordering from append order in `ActionTraceSink.events()`. That is deterministic today, but it is implicit rather than modeled. The engine should expose ordering as a proper trace contract instead of relying on vector position as an undocumented proxy.

## Assumption Reassessment (2026-03-19)

1. Current action trace shape is defined in `crates/worldwake-sim/src/action_trace.rs` via `ActionTraceEvent { tick, actor, def_id, action_name, kind }` and `ActionTraceSink { events: Vec<ActionTraceEvent> }`. There is no `sequence_in_tick`, ordinal, or explicit ordering helper beyond append order.
2. Current trace emission happens in `crates/worldwake-sim/src/tick_step.rs` through repeated `runtime.record_action_trace(ActionTraceEvent { ... })` calls in the input, active-action progress, and abort paths. This means the runtime already has a single append path where a stable ordering field can be assigned without adding cross-system coupling.
3. Current action-trace focused coverage exists for emptiness, actor/tick filtering, latest committed lookup, and summaries in `crates/worldwake-sim/src/action_trace.rs`, but there is no focused/unit test proving or naming an explicit same-tick cross-agent ordering contract.
4. Existing docs point people to `events_at()` and `events_for_at()` for same-tick visibility, for example in `AGENTS.md` and `docs/golden-e2e-testing.md`, but they do not document how to prove relative order among multiple same-tick events from different actors.
5. The completed golden in `archive/tickets/completed/S14CONMEME-001-same-place-office-fact-still-requires-tell.md` exposed this gap directly: same-place `Tell` can lawfully unlock downstream `declare_support` in the same tick, and the test had to assert append order from `ActionTraceSink.events()` instead of a first-class trace field.
6. Intended verification layers for this ticket are focused/unit coverage in `worldwake-sim` for the trace substrate, plus a narrow integration check in `worldwake-ai` only if implementation changes consumer expectations. This is not primarily an AI-candidate or planner ticket.
7. Ordering contract is action lifecycle ordering within a single tick. The compared branches are not symmetric in the motivating golden because one actor's committed `Tell` mutates another actor's belief state before that second actor commits `declare_support`, but the trace substrate should be general and not encode social/political specifics.
8. This ticket is not removing or weakening any heuristic. It is formalizing an ordering substrate that tests and debugging already rely on implicitly, which is cleaner than continuing to depend on undocumented append behavior.

## Architecture Check

1. Adding a first-class intra-tick ordering field to `ActionTraceEvent` is cleaner than sprinkling ad hoc helper code through tests because ordering belongs to the trace model itself, not to each consumer.
2. The implementation should preserve the append-only nature of traces and determinism while making the contract explicit. A monotonic per-tick ordinal or equivalent is preferable to global mutable counters hidden outside the trace sink.
3. No backward-compatibility aliasing or duplicate trace paths should be introduced. Existing query methods should be updated or extended in place so there remains one canonical action-trace substrate.

## Verification Layers

1. `ActionTraceEvent` carries explicit same-tick ordering metadata and preserves deterministic append semantics -> focused/unit tests in `crates/worldwake-sim/src/action_trace.rs`
2. `step_tick()` assigns monotonically increasing per-tick order to all action lifecycle events, including start, commit, abort, and start-failure paths -> focused/unit or integration coverage in `worldwake-sim`
3. Same-tick cross-agent consumers can assert causal order without relying on vector index position -> focused/unit coverage first; golden consumption checks only if needed
4. Determinism of the trace model remains intact after serialization-free runtime use -> focused/unit coverage, not indirect golden-only inference

## What to Change

### 1. Extend the action trace model with explicit ordering metadata

Update `crates/worldwake-sim/src/action_trace.rs` so `ActionTraceEvent` carries a first-class intra-tick ordering field such as `sequence_in_tick` or an equivalent explicit ordinal. Keep the contract append-only and deterministic. Add query helpers only if they materially reduce duplicate consumer logic, for example retrieving ordered events for a tick or exposing a helper for comparing two committed events.

### 2. Assign ordering at the single runtime emission boundary

Update `crates/worldwake-sim/src/tick_step.rs` and any supporting runtime plumbing so every emitted action trace event receives explicit ordering metadata at record time. The ordering source must be centralized and canonical; do not let individual action handlers or systems invent their own ordinals.

### 3. Add focused coverage for the new substrate

Strengthen `worldwake-sim` tests so they prove:
- ordering metadata is monotonic within a tick,
- ordering resets or scopes correctly across ticks if it is per-tick,
- mixed lifecycle events in one tick preserve causal order,
- cross-actor same-tick sequences can be asserted without using vector position as the contract.

## Files to Touch

- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `AGENTS.md` (review after implementation for updated action-trace guidance)
- `docs/golden-e2e-testing.md` (review after implementation for assertion-surface guidance)

## Out of Scope

- Changing simulation semantics to force next-tick cross-agent propagation
- Adding social or political system delays to avoid same-tick chains
- Decision-trace provenance work beyond what is strictly necessary to keep trace consumers coherent
- Backward-compatibility wrappers or duplicate trace sinks

## Acceptance Criteria

### Tests That Must Pass

1. New focused `worldwake-sim` tests proving explicit same-tick ordering on `ActionTraceEvent`
2. Existing suite: `cargo test -p worldwake-sim`
3. Existing suite: `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`
4. Existing suite: `cargo clippy --workspace`

### Invariants

1. Action traces remain append-only and deterministic.
2. Same-tick ordering becomes a first-class trace contract instead of an implicit property of vector position.
3. Ordering metadata is assigned centrally by the runtime, not by individual systems or action handlers.
4. No simulation-law change is introduced; only observability and assertion strength improve.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_trace.rs` — add focused tests for explicit intra-tick ordering fields and helpers.
2. `crates/worldwake-sim/src/tick_step.rs` — add focused or integration-level tests proving order assignment across mixed action lifecycle events in one tick.
3. `crates/worldwake-ai/tests/golden_emergent.rs` — update only if necessary to assert on the new explicit ordering field instead of raw append index.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`
3. `cargo clippy --workspace`
