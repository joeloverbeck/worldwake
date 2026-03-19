# ACTTRCPAY-001: Structured Action Trace Details For Identity-Sensitive Actions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` action trace data model and recording path, `worldwake-ai` golden assertions that can consume the richer trace surface
**Deps**: `archive/tickets/ACTEXETRA-001-action-trace-types-and-sink.md`, `archive/tickets/ACTEXETRA-003-record-trace-events-at-hook-points.md`, `archive/tickets/completed/S14CONMEME-001-same-place-office-fact-still-requires-tell.md`, `archive/tickets/completed/S14CONMEMEME-002-already-told-recent-subject-does-not-crowd-out-untold-office-fact.md`, `specs/S14-conversation-memory-emergence-golden-suites.md`, `docs/golden-e2e-testing.md`

## Problem

Current action traces can prove that a `tell` action started or committed and can order it relative to downstream actions such as `declare_support`, but they do not expose which subject was told. That weakens cross-layer traceability for social-to-political causal chains and forces goldens to fall back to indirect state inspection where the action trace should be able to speak directly.

## Assumption Reassessment (2026-03-19)

1. The current action trace model in `crates/worldwake-sim/src/action_trace.rs` stores `tick`, `sequence_in_tick`, `actor`, `def_id`, `action_name`, and `kind`, but no structured payload detail. `ActionTraceEvent::summary()` therefore cannot report `tell` subject identity today.
2. `tell` payload identity already exists authoritatively in `crates/worldwake-sim/src/action_payload.rs` as `TellActionPayload { listener, subject_entity }`, and the live action path in `crates/worldwake-systems/src/tell_actions.rs` uses that payload for validation and commit. The missing substrate is trace recording, not action semantics.
3. Current focused/runtime trace coverage around action ordering exists in `crates/worldwake-sim/src/action_trace.rs` (`summary_format_covers_all_variants()`, `record_assigns_explicit_sequence_per_tick_even_when_ticks_interleave()`) and `crates/worldwake-sim/src/tick_step.rs` (`action_trace_assigns_explicit_order_across_started_failed_and_committed_events()`, `action_trace_exposes_same_tick_cross_actor_commit_order()`), but those tests only prove lifecycle ordering and generic summaries, not payload-level detail.
4. Current golden E2E coverage uses action traces heavily for lifecycle ordering in `crates/worldwake-ai/tests/golden_emergent.rs`, `crates/worldwake-ai/tests/golden_offices.rs`, and `crates/worldwake-ai/tests/golden_social.rs`. In particular, `golden_same_place_office_fact_still_requires_tell`, `golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact`, and `golden_tell_propagates_political_knowledge` can only assert generic `tell` ordering today, not subject-specific `tell` ordering. The crowd-out golden currently proves the branch indirectly by counting two committed `tell` events before office belief arrival and then ordering the second generic `tell` before downstream `declare_support`.
5. `docs/golden-e2e-testing.md` already distinguishes action traces from decision traces, but `specs/S14-conversation-memory-emergence-golden-suites.md` Scenario 25 still describes a subject-specific action-trace assertion that the current trace surface cannot express. This ticket should close that architecture/documentation mismatch by improving the trace substrate rather than weakening the intended observability standard.
6. Ordering is still action lifecycle ordering via `(tick, sequence_in_tick)`. The compared branches here are not symmetric: subject A and subject B differ because the divergence is driven by social suppression/filtering before truncation, while `declare_support` is a downstream political effect. This ticket does not change that ordering contract; it makes the social side inspectable at the action layer.
7. Mismatch correction: the missing architecture is not “more tests” by itself. The missing architecture is payload-aware action tracing for identity-sensitive actions, beginning with `tell` and designed as an extensible trace-detail surface rather than an ad hoc `tell`-only string.
8. Mismatch correction: one dependency path in the original ticket was stale. The completed same-place S14 ticket is archived at `archive/tickets/completed/S14CONMEME-001-same-place-office-fact-still-requires-tell.md`, not `S14CONMEMEME-001`.

## Architecture Check

1. Extending `ActionTraceEvent` with optional structured action-detail data is cleaner than forcing goldens to infer subject identity from side effects or by stitching decision traces and listener belief state together for every social assertion. It improves observability without altering world meaning.
2. The design should be extensible across identity-sensitive actions such as `tell`, `declare_support`, `threaten`, or `bribe`, but it should start with a small enum or structured field rather than bespoke per-test formatting. No backwards-compatibility aliasing or secondary shadow trace types should be introduced.

## Verification Layers

1. `ActionTraceEvent` records subject-identity detail for `tell` commits and starts -> focused `worldwake-sim` unit/runtime tests over `action_trace.rs` and `tick_step.rs`
2. Same-tick and cross-tick ordering remains inspectable through `(tick, sequence_in_tick)` while preserving payload detail -> focused `tick_step.rs` tests
3. Social goldens can assert which `tell` subject was committed before downstream office actions -> `worldwake-ai` golden E2E tests using action traces
4. Delayed office installation is not a proxy for earlier social ordering here; authoritative world state still proves the final office outcome, while action traces prove which `tell` payload happened first

## What to Change

### 1. Add structured action-trace detail data

Extend `crates/worldwake-sim/src/action_trace.rs` so `ActionTraceEvent` can carry optional structured detail for identity-sensitive actions. The representation should be typed and extensible, not a test-only string field.

Minimum first-class support in this ticket:
- `tell` detail containing at least `listener` and `subject`

Optional if the design remains clean and local:
- `declare_support`, `threaten`, or `bribe` details using the same trace-detail abstraction

### 2. Record trace details from the live action path

Thread the relevant payload-derived detail into the action-trace recording path in `crates/worldwake-sim/src/tick_step.rs` without changing action execution semantics.

The recording path must preserve the current zero-cost behavior when tracing is disabled.

### 3. Strengthen focused trace coverage

Add or update focused tests in `crates/worldwake-sim/src/action_trace.rs` and/or `crates/worldwake-sim/src/tick_step.rs` to prove:
- detail is present for `tell`
- ordering keys are unchanged
- summary/debug helpers remain deterministic and legible

### 4. Upgrade one golden to consume the richer surface

Use the new trace detail in the existing crowd-out golden in `crates/worldwake-ai/tests/golden_emergent.rs` so it can assert subject-specific `tell` ordering directly instead of relying only on decision traces plus listener belief state.

## Files to Touch

- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `crates/worldwake-sim/src/action_payload.rs` (modify only if needed to support clean trace-detail conversion; not expected)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)

## Out of Scope

- Changing `tell` behavior, conversation-memory rules, or political planning behavior
- Replacing decision traces with action traces for AI reasoning assertions
- Adding generic string blobs or ad hoc debug-only fields instead of structured trace detail
- Retrofitting every action in the engine with trace detail in one ticket

## Acceptance Criteria

### Tests That Must Pass

1. A focused `worldwake-sim` test proving `tell` action traces expose subject identity
2. `cargo test -p worldwake-sim action_trace`
3. `cargo test -p worldwake-sim tick_step`
4. `cargo test -p worldwake-ai --test golden_emergent golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact -- --exact`
5. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Action traces remain append-only lifecycle records keyed by `(tick, sequence_in_tick)`; payload detail augments the trace and does not replace the ordering key.
2. The new trace detail reflects the actual action payload and does not invent or infer subject identity from downstream state.
3. Tracing disabled must remain behaviorally identical to current execution.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_trace.rs` — add focused tests for structured action-trace detail and summary rendering.
2. `crates/worldwake-sim/src/tick_step.rs` — strengthen runtime trace tests so `tell` records the expected payload detail without disturbing ordering semantics.
3. `crates/worldwake-ai/tests/golden_emergent.rs` — tighten `golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact` to assert subject-specific `tell` ordering directly from action traces.

### Commands

1. `cargo test -p worldwake-sim action_trace`
2. `cargo test -p worldwake-sim tick_step`
3. `cargo test -p worldwake-ai --test golden_emergent golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact -- --exact`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-19
- What changed:
  - Added optional typed `ActionTraceDetail` data to `ActionTraceEvent` in `crates/worldwake-sim/src/action_trace.rs` with first-class `tell { listener, subject }` support and deterministic summary rendering.
  - Threaded payload-derived trace detail through the live `tick_step` recording path so started, committed, and aborted lifecycle events preserve identity-sensitive payload detail without changing action semantics or tracing-disabled behavior.
  - Tightened `crates/worldwake-ai/tests/golden_emergent.rs` so `golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact` asserts the office-unlocking tell by subject-specific action-trace detail instead of relying on generic tell counting alone.
  - Corrected the stale dependency path for `S14CONMEME-001` during ticket reassessment.
- Deviations from original plan:
  - The implementation kept the trace detail surface intentionally narrow: only `tell` received first-class typed detail in this ticket.
  - The runtime plumbing records typed detail on aborted events as well as started and committed events because the active `ActionInstance` already carries the authoritative payload and the extra observability came essentially for free.
  - `crates/worldwake-sim/src/action_payload.rs` did not require changes; payload-to-trace-detail extraction stayed local to the trace layer.
- Verification results:
  - Passed `cargo test -p worldwake-sim action_trace`
  - Passed `cargo test -p worldwake-sim tick_step`
  - Passed `cargo test -p worldwake-sim`
  - Passed `cargo test -p worldwake-ai --test golden_emergent golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact -- --exact`
  - Passed `cargo test -p worldwake-ai --test golden_emergent`
  - Passed `cargo clippy --workspace --all-targets -- -D warnings`
