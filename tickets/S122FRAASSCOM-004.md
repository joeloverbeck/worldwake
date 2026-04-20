# S122FRAASSCOM-004: Trace surface payload + integration tests + survival-golden gate

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `FrameTransitionKind::Cleared` gains `failed_assumption: Option<FrameAssumption>` field; `format_frame_transition_kind` surfaces the payload in the trace summary; integration tests #10 (suppression prevents re-adoption) and #11 (stale defers, fresh refutes) land; survival goldens (`baseline`, `contested`, `scattered`) re-run as the architectural acceptance gate.
**Deps**: archive/tickets/S122FRAASSCOM-003.md

## Problem

With the assumption evaluable (003), the failure event is recorded but the decision-trace summary cannot answer "the agent abandoned this plan because it now believes there is no Apple at Fertile Fields." This ticket extends the `Cleared` trace variant to carry the failed-assumption payload, lands the suppression-prevents-re-adoption (#10) and stale-defers (#11) integration tests, and re-runs the survival goldens to verify the architectural fix resolves the Camp ↔ Fertile Fields oscillation cited in the spec's Motivating Evidence.

## Assumption Reassessment (2026-04-20)

1. Existing focused/unit coverage in `crates/worldwake-ai/src/decision_trace.rs#[cfg(test)]`: tests at lines 4234 (Created), 4249 (Progressed), 4257 (Suspended), 4269 (Resumed), 4277 (Exhausted), and one for `Cleared` cover `format_frame_transition_kind`. No existing test asserts payload on `Cleared`. Adding `failed_assumption: Option<FrameAssumption>` to the `Cleared` variant is a struct-variant field addition — every existing construction site must add `failed_assumption: None` (no implicit default for struct-variant fields). Grep for `FrameTransitionKind::Cleared` callers in worldwake-ai found two production sites: `agent_tick/mod.rs:1317-1320` (assumption-driven emission) and `decision_trace.rs:1992` (format match arm), plus existing test construction sites in `decision_trace.rs#[cfg(test)]`. Other call sites (e.g., `frame.rs:336` setting `runtime.last_frame_clear_reason`) work via `FrameClearReason`, not `FrameTransitionKind`, and are not affected.
2. Spec deliverable D6 in `specs/S122-frame-assumption-commodity-availability.md` (lines 218–234). Integration tests #10 (lines 249–251) and #11 (lines 250–251) defined. Survival goldens #14 (lines 257–259) and #15 (lines 260–261) defined. The `explorer_discovers_food_source` regression guard at `crates/worldwake-ai/tests/golden_survival_baseline.rs:382` is named in #15.
3. Shared abstraction boundary under audit: `FrameTransitionKind::Cleared` variant shape (`crates/worldwake-ai/src/decision_trace.rs:58-60`) and `format_frame_transition_kind` (`decision_trace.rs:1965`). The boundary is the variant struct and its formatter.
6. Intended layer: AI runtime / decision-trace layer for D6 (focused unit coverage); AI runtime for integration tests #10 and #11 (full action registries required because the tests exercise plan adoption, candidate ranking, and discrepancy-driven suppression); golden E2E for #14 and #15 (full simulation, full action registries, ignored-by-default release runs).
12. Survival baselines named in `crates/worldwake-ai/tests/`: `golden_survival_baseline.rs`, `golden_survival_contested.rs`, `golden_survival_scattered.rs`. The `explorer_discovers_food_source` test in `golden_survival_baseline.rs:382` already passes after S109's TYPDISTAX-004 corrections — S122 must not regress it. The architectural acceptance is that the Camp ↔ Fertile Fields oscillation cited in the spec's Motivating Evidence (Agent B last eats at tick 578, then enters 591-tick oscillation) is resolved without TTL widening or contract relaxation.
13. Choice between extending `FrameTransitionKind::Cleared` vs. adding a new variant `AssumptionExhausted`: `Cleared` is what `emit_assumption_transitions` already emits on `CriticalFailure` (mod.rs:1317–1320), so extending the existing variant is a smaller diff than adding a sibling. `Exhausted` is emitted from `check_patience_exhaustion` (frame.rs:385) for patience exhaustion, not from assumption failure — leaving it untouched preserves clear semantic separation.

## Architecture Check

1. Adding `failed_assumption: Option<FrameAssumption>` to the existing `Cleared` variant keeps the trace surface backward-compatible at the source level — every existing emission can pass `None`. Only the assumption-failure emission populates `Some(payload)`. This is cleaner than introducing a sibling variant `AssumptionExhausted { ... }` because all "frame cleared" trace data lives under one variant, and the formatter can branch on the optional payload.
2. `format_frame_transition_kind` reads the optional payload and includes `(commodity, place)` in the summary string when present. No new field on `IntentionFrame` (the payload travels with the transition record, not the frame). FND-27 satisfied: the trace summary remains a derived view, recomputable from the structured transition record.

## Verification Layers

1. `Cleared { reason: AssumptionFailed, failed_assumption: Some(CommodityAvailableAt { commodity, place }) }` formats to a human-readable string naming `(commodity, place)` -> focused unit test in `decision_trace.rs#[cfg(test)]`.
2. `Cleared { reason: ..., failed_assumption: None }` formats without an assumption suffix (backward-compatible) -> existing `format_frame_transition_*` tests updated to pass `None` and continue to assert the same output.
3. Suppression-prevents-re-adoption (test #10): after assumption failure at tick T, the agent must not re-adopt the same `(goal, place)` opportunity for the next `structural_block_ticks` ticks -> integration test asserts via `runtime.current_plan` (no plan with the suppressed goal) and `DiscrepancyMemory` (entry still active, `expires_tick > current_tick`). Verified at the `agent_tick` runtime layer with full action registries.
4. Stale-defers / fresh-refutes (test #11): agent with stale belief about Apple at P retains the intention until co-located perception refreshes; on first co-located tick, the assumption fails -> integration test verifies the deferred → critical-failure transition.
5. Survival goldens (#14 + #15): all three `golden_survival_*` tests pass within their authored `max_authored_critical_run_ticks` bounds. The Camp ↔ Fertile Fields oscillation is resolved architecturally -> golden E2E with `--ignored --test-threads=1`.
6. Multi-layer ticket — trace formatting (focused unit), runtime suppression (integration), end-to-end behavior (golden E2E). Each invariant maps to its own surface; no collapsing into one assertion.

## What to Change

### 1. Extend `FrameTransitionKind::Cleared` with `failed_assumption` field

- File: `crates/worldwake-ai/src/decision_trace.rs` (lines 58–60)
- Change:

  ```rust
  Cleared {
      reason: FrameClearReason,
      failed_assumption: Option<FrameAssumption>,
  },
  ```

- Add `FrameAssumption` to the file's `use worldwake_core::{...}` imports.

### 2. Update `format_frame_transition_kind` to surface the payload

- File: `crates/worldwake-ai/src/decision_trace.rs` (line 1965, the `Cleared` arm at line 1992)
- When `failed_assumption` is `Some(FrameAssumption::CommodityAvailableAt { commodity, place })`, append a suffix to the formatted string naming the commodity and place (format: `cleared({reason:?}) — failed: CommodityAvailableAt(commodity={commodity:?}, place={place:?})`).
- For `Some(FrameAssumption::TargetAlive(entity))`, similar suffix naming the entity.
- For `Some(FrameAssumption::RouteExists { from, to })` and `Some(NoCriticalThreat)`, similar concise suffixes.
- For `None`, omit the suffix (preserves the existing format).

### 3. Update the assumption-driven emission site to populate the payload

- File: `crates/worldwake-ai/src/agent_tick/mod.rs` (lines 1317–1320)
- Change `AssumptionEvalResult::CriticalFailure(_) =>` to `AssumptionEvalResult::CriticalFailure(failed) =>` and emit:

  ```rust
  ft.push(FrameTransitionKind::Cleared {
      reason: FrameClearReason::AssumptionFailed,
      failed_assumption: Some(*failed),
  });
  ```

### 4. Update other emission sites for backward-compatible `None`

- Files: any other site that constructs `FrameTransitionKind::Cleared { reason: ... }`. Grep workspace before implementation: likely candidates are `crates/worldwake-cli/src/bin/observer.rs` and `crates/worldwake-ai/tests/golden_harness/` (e.g., `survival_forensics_assertions.rs` had a `frame_transition: None` reference seen in the reassessment).
- Add `failed_assumption: None` to each construction.

### 5. Add integration test #10 (suppression prevents re-adoption)

- File: `crates/worldwake-ai/src/agent_tick/tests.rs`
- New test `commodity_assumption_failure_suppresses_readoption`:
  - Setup as in S122FRAASSCOM-003's test #9 (extracted into a shared helper if duplication is significant).
  - After the assumption fails and discrepancy is recorded at tick T, step the agent for `structural_block_ticks` ticks.
  - At each step, assert: the agent does NOT adopt a plan whose committed goal matches the suppressed `(AcquireCommodity { Apple, ... }, place: P)` pair (check `runtime.current_plan` ranked candidates).
  - Assert: the agent considers and adopts an alternative plan when one is available (Harvest at the same place if a resource source is configured, or Travel to a different place with a known source).

### 6. Add integration test #11 (stale defers, fresh refutes)

- File: `crates/worldwake-ai/src/agent_tick/tests.rs`
- New test `commodity_assumption_stale_defers_fresh_refutes`:
  - Construct world: agent A at place A, no item lots for Apple anywhere in the world.
  - Establish a stale belief in A's `AgentBeliefStore` that an Apple lot exists at P (recorded with old `last_observed_tick`, `last_known_inventory[Apple] > 0`).
  - Adopt `Travel(P) → pick_up(...)` plan via the standard adoption path.
  - Step the agent (still at A or in transit). Assert: assumption defers (intention persists; no `CriticalFailure`).
  - Step until co-located at P. Assert: on the first co-located tick, assumption returns `CriticalFailure(FrameAssumption::CommodityAvailableAt { commodity: Apple, place: P })`.

### 7. Update existing decision-trace tests to construct the new field

- File: `crates/worldwake-ai/src/decision_trace.rs#[cfg(test)]`
- Update any test that constructs a `Cleared` variant to pass `failed_assumption: None` (or `Some(...)` where the new behavior is being tested).
- Add a new test `format_frame_transition_cleared_with_failed_commodity_assumption_includes_payload` asserting the formatter output names `(commodity, place)`.

### 8. Survival goldens regression check

- Files: `crates/worldwake-ai/tests/golden_survival_baseline.rs`, `golden_survival_contested.rs`, `golden_survival_scattered.rs` — no code changes. Re-run as the acceptance gate.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add field, update formatter, update tests, add new test)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — emission site at line 1317)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — new integration tests #10 and #11)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — update any `Cleared` construction site to `failed_assumption: None`)
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` (modify if it constructs `Cleared` variants)

## Out of Scope

- Falsification probes (#16, #17, #18) — S122FRAASSCOM-005.
- Modifying `IntentionFrame` schema — the payload travels with the trace record, not the frame.
- Adding event-log emission for assumption failures — that is S110 work, not S122. S122 itself does not add any new `EventTag` variant.
- Surfacing other `FrameAssumption` variants in the trace beyond what the formatter naturally produces (the spec scopes the trace upgrade to making the failure identity nameable).

## Acceptance Criteria

### Tests That Must Pass

1. New: `format_frame_transition_cleared_with_failed_commodity_assumption_includes_payload` — formatter surfaces `(commodity, place)`.
2. Updated: existing `format_frame_transition_*` tests pass with `failed_assumption: None` defaults.
3. New integration: `commodity_assumption_failure_suppresses_readoption` (test #10) — agent does not re-adopt suppressed `(goal, place)` for `structural_block_ticks` ticks.
4. New integration: `commodity_assumption_stale_defers_fresh_refutes` (test #11) — stale belief defers, co-located perception refutes.
5. Survival goldens: `golden_survival_baseline`, `golden_survival_contested`, `golden_survival_scattered` all pass within their `max_authored_critical_run_ticks` bounds (regression check #14).
6. `explorer_discovers_food_source` (in `golden_survival_baseline.rs:382`) remains green — no regression from S109 corrections (regression check #15).
7. No new `EventTag` variant introduced — `git diff crates/worldwake-core/src/event_tag.rs` shows zero changes (validation §13a).
8. Existing suite: `cargo test -p worldwake-ai` passes.

### Invariants

1. The decision trace records the failed-assumption identity for every `CriticalFailure`-driven clear. (FND-29.)
2. The Camp ↔ Fertile Fields oscillation cited in the spec's Motivating Evidence is resolved through the assumption-failure → suppression chain, not through TTL widening or contract relaxation. (FND-1, FND-21.)
3. `FrameTransitionKind::Cleared` payload is informational (trace-only); modifying it does not change runtime decision behavior. (FND-27.)
4. No new authoritative state added; `EventTag` enum unchanged. (FND-28 boundary preserved between S122 and the future S110 event-log work.)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs#[cfg(test)]` — 1 new formatter test; existing tests updated for the new `failed_assumption: None` field.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — 2 new integration tests (#10 and #11).

### Commands

1. `cargo test -p worldwake-ai --lib decision_trace`
2. `cargo test -p worldwake-ai --lib agent_tick`
3. `cargo test --release -p worldwake-ai --test golden_survival_baseline -- --ignored --test-threads=1`
4. `cargo test --release -p worldwake-ai --test golden_survival_contested -- --ignored --test-threads=1`
5. `cargo test --release -p worldwake-ai --test golden_survival_scattered -- --ignored --test-threads=1`
6. `cargo clippy --workspace --all-targets -- -D warnings`
