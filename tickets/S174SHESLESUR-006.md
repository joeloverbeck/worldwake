# S174SHESLESUR-006: Failed-rest forensics — FailedRestOpportunity records + ActionTraceDetail::SleepInterrupted population

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `FailedRestOpportunity` and `FailedRestKind` types in `worldwake-ai/src/survival_forensics.rs`; new `failed_rest_opportunities` field on `CriticalWindowFrame`; population of `ActionTraceDetail::SleepInterrupted` at the sleep-abort trace boundary
**Deps**: 001 (SleepFailureCause enum, ActionTraceDetail::SleepInterrupted variant), 003 (belief-view accessors for failure attribution at candidate-rejection sites)

## Problem

S174 D8 requires `SurvivalForensicExtractor` to capture failed-rest opportunities during active critical fatigue windows so a future reader can answer "this agent collapsed from exhaustion because it failed to rest N times for these specific reasons." Currently `CriticalWindowFrame` (`crates/worldwake-ai/src/survival_forensics.rs:30-40`) captures exhaustion state and blocker summary but has no typed surface for failed-rest events. Additionally, ticket 001 added the `ActionTraceDetail::SleepInterrupted` variant; this ticket populates it at the abort-trace boundary in `tick_step.rs::abort_trace_detail_for_instance`, redirecting sleep aborts from the existing `SelfCareInterrupted { kind: Sleep, ... }` path.

The paired spec S175 will read `CriticalWindowReport.failed_rest_opportunities` to prove "fatigue collapse follows N failed-rest opportunities."

## Assumption Reassessment (2026-05-26)

1. Verified current code: `SurvivalForensicExtractor` at `crates/worldwake-ai/src/survival_forensics.rs:157-161` aggregates active_windows + completed_reports. `CriticalWindowFrame` at lines 30-40 carries `tick, need_value, selected_goal, selected_plan_source, top_competitors, active_action, exhaustion_state, blocker_summary, local_authoritative_summary`. `CriticalWindowReport` at lines 19-27 carries `agent, need, start_tick, end_tick, threshold, peak_value, frames`. Both derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `abort_trace_detail_for_instance` at `crates/worldwake-sim/src/tick_step.rs:68-100` is the trace-emission boundary that maps abort events to `ActionTraceDetail` variants; currently emits `SelfCareInterrupted { kind: Sleep, basin: None }` for sleep aborts.
2. Spec assumption verified against S174 D7 and D8. The Implementation choice in D7 ("the abort helper emits `SleepInterrupted` for sleep actions and `SelfCareInterrupted` for the other five families") is a redirection — this ticket implements that switch.
3. Shared abstraction boundary under audit: forensic record schema (`CriticalWindowFrame` field extension) + action-trace boundary (`abort_trace_detail_for_instance` variant routing). The forensic recorder reads from the action-trace + event-log streams to construct `FailedRestOpportunity` entries; the action-trace boundary feeds the structured cause via the new `SleepInterrupted` variant.
4. The `failed_rest_opportunities: Vec<FailedRestOpportunity>` field on `CriticalWindowFrame` needs `#[serde(default)]` so existing serialized critical-window snapshots continue to deserialize (rides ticket 001's `SAVE_FORMAT_VERSION 107→108` bump without bumping again).
5. Three populating paths: (a) sleep aborts mid-episode → write `FailedRestOpportunity { kind: Interrupted { cause }, ... }` from the existing abort handler stack; (b) sleep action start fails the rest-site precondition → write `FailedRestOpportunity { kind: PreconditionRejected, ... }` from the `BestEffort` start-failure path; (c) actor abandons a Sleep intention for another need during a critical window → write `FailedRestOpportunity { kind: PreemptedByHigherNeed { need }, ... }` from the planner's intention-revision path. Path (a) is straightforward; (b) needs hookup at the action-start failure boundary; (c) requires reading intent transitions which is more nuanced and may be deferred to a follow-up if architectural reach is uncertain.
6. Mismatch + correction: the existing `SelfCareInterrupted { kind: Sleep, basin: None }` path stays usable for trace-sink consumers that do not care about the structured cause. The new `SleepInterrupted` variant carries strictly more information (place, cause, accumulated_recovery, was_rough_sleep). The abort helper picks `SleepInterrupted` for sleep aborts; existing tests asserting `SelfCareInterrupted { kind: Sleep, ... }` need updates to assert `SleepInterrupted { ... }` instead.

## Architecture Check

1. Two new types (`FailedRestOpportunity`, `FailedRestKind`) are introduced in `worldwake-ai/src/survival_forensics.rs` — the same module that owns `CriticalWindowFrame`. Locating the types here (rather than in `worldwake-sim` or `worldwake-core`) keeps the forensic schema crate-private to `worldwake-ai`. The types are derived per-decision records (FND-27 derived view), not authoritative state — they aggregate event-log + action-trace data into a forensic snapshot.
2. Routing sleep aborts to `ActionTraceDetail::SleepInterrupted` (instead of the existing `SelfCareInterrupted { kind: Sleep }`) preserves FND-28 — one structured cause surface, not two. The `SelfCareInterrupted { kind: Sleep }` path is removed for the sleep-abort case; `SelfCareInterrupted` continues to handle the other five self-care families per S173.
3. The forensic record reads from authoritative event-log + trace streams (FND-26: systems via state); the recorder does not subscribe to the planner directly. This keeps the failure-attribution discipline decoupled from planning logic.

## Verification Layers

1. `FailedRestOpportunity` and `FailedRestKind` types compile with required derives (`Clone, Debug, PartialEq, Eq, Serialize, Deserialize`) -> focused unit test in `survival_forensics.rs`
2. `CriticalWindowFrame.failed_rest_opportunities: Vec<FailedRestOpportunity>` with `#[serde(default)]` round-trips correctly -> serde round-trip test
3. Sleep abort mid-episode populates `ActionTraceDetail::SleepInterrupted { place, cause, accumulated_recovery, was_rough_sleep }` -> action-trace assertion via integration test
4. The existing `SelfCareInterrupted { kind: Sleep, basin: None }` path is no longer emitted for sleep aborts -> action-trace assertion (negative branch — verify no SelfCareInterrupted-with-Sleep events fire)
5. `FailedRestOpportunity { kind: Interrupted { cause } }` is appended to the active critical window's `failed_rest_opportunities` when sleep aborts during a critical-fatigue window -> integration test exercising forensic extractor
6. `FailedRestOpportunity { kind: PreconditionRejected }` is appended when sleep action start fails the rest-site precondition during a critical window -> integration test exercising the start-failure path
7. `FailedRestOpportunity { kind: PreemptedByHigherNeed { need } }` is appended when the actor abandons a Sleep intention for a higher-priority need (may be deferred per Assumption 5)

## What to Change

### 1. Add `FailedRestOpportunity` and `FailedRestKind` types

In `crates/worldwake-ai/src/survival_forensics.rs`, alongside `CriticalWindowFrame`:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedRestOpportunity {
    pub tick: Tick,
    pub place: EntityId,
    pub kind: FailedRestKind,
    pub was_rough: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailedRestKind {
    /// Sleep started but was interrupted mid-episode.
    Interrupted { cause: SleepFailureCause },
    /// Sleep candidate was emitted but precondition rejected at start
    /// (rest site became full between candidate emission and arrival).
    PreconditionRejected,
    /// Sleep candidate was emitted but the actor was preempted by a
    /// higher-priority need before reaching the rest site.
    PreemptedByHigherNeed { need: HomeostaticNeedId },
}
```

Import `SleepFailureCause` from `worldwake_core::decision_event_payload` and `HomeostaticNeedId` from `worldwake_core::needs`.

### 2. Extend `CriticalWindowFrame` with `failed_rest_opportunities`

In `survival_forensics.rs:30-40`, add the new field:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CriticalWindowFrame {
    // ... existing fields ...
    #[serde(default)]
    pub failed_rest_opportunities: Vec<FailedRestOpportunity>,
}
```

`#[serde(default)]` ensures pre-bump serialized snapshots continue to deserialize.

### 3. Populate `ActionTraceDetail::SleepInterrupted` at the abort boundary

In `crates/worldwake-sim/src/tick_step.rs:68-100` (`abort_trace_detail_for_instance`), redirect sleep aborts to emit `SleepInterrupted` instead of `SelfCareInterrupted { kind: Sleep, basin: None }`:

- When the aborted action is `sleep`, construct `ActionTraceDetail::SleepInterrupted { place, cause, accumulated_recovery, was_rough_sleep }` where:
  - `place` is read from the aborted `SleepEpisode.place` (existing accessor).
  - `cause` is the `SleepFailureCause` supplied by the abort path (defaults to `Generic` if the abort handler did not supply a specific cause; ticket 004 refines the cause supply at `abort_sleep_episode`).
  - `accumulated_recovery` is read from the aborted `SleepEpisode.accumulated_recovery`.
  - `was_rough_sleep` is derived from whether the actor's effective place had `RestCapacity` AND whether the action's `ActionState::Sleep { rough }` (or equivalent per ticket 004) was `true`.

For the other five self-care families (`eat`, `drink`, `toilet`, `wash`, `relieve_wilderness`), continue emitting `SelfCareInterrupted` per S173. The match in `abort_trace_detail_for_instance` is conditional-return; just redirect the `sleep` branch.

### 4. Hook forensic recording at three failed-rest paths

In `survival_forensics.rs` (within or adjacent to the existing active-window event processing), wire three new populating paths:

- **(a) Sleep aborts mid-episode**: subscribe to action-trace events for `ActionTraceDetail::SleepInterrupted`. For each event, if any active critical-fatigue window for the focal agent is active, append a `FailedRestOpportunity { tick, place, kind: Interrupted { cause }, was_rough: was_rough_sleep }` to the window's frame.
- **(b) Sleep action start failures**: subscribe to `EventTag::ActionAborted` events with the `sleep` action name. If the abort fires at action-start (before commit), and any active critical-fatigue window is active, append `FailedRestOpportunity { tick, place, kind: PreconditionRejected, was_rough: false }`. Distinguish start-failure from mid-episode-abort via action-trace context (existing `ActionTraceKind::Aborted` carries the `instance_id` which can be cross-referenced).
- **(c) Preemption by higher-priority need (deferred)**: this path requires reading intention transitions. If the architectural reach is uncertain at ticket-implementation time, leave a TODO marker referencing this ticket and defer to a follow-up scoped to ticket 011 (Scenario E) which is the consumer that requires this record. Document the deferral explicitly in the ticket's Outcome section.

## Files to Touch

- `crates/worldwake-ai/src/survival_forensics.rs` (modify — add `FailedRestOpportunity` + `FailedRestKind` types, extend `CriticalWindowFrame`, hook 2-3 populating paths)
- `crates/worldwake-sim/src/tick_step.rs` (modify — redirect sleep-abort trace detail to `SleepInterrupted` variant in `abort_trace_detail_for_instance`)
- Likely: existing tests in `survival_forensics.rs` (verify via inline-test boundary at line 483; extend tests to cover the new field and population paths)
- Likely: existing tests asserting `ActionTraceDetail::SelfCareInterrupted { kind: SelfCareUseKind::Sleep, ... }` for sleep aborts — locate via `grep -rn "SelfCareInterrupted.*Sleep\|SelfCareUseKind::Sleep" crates/worldwake-ai/tests/ crates/worldwake-sim/tests/`. Update each to assert `SleepInterrupted { ... }` instead.

## Out of Scope

- No new `EventTag` variant (FND-28: enrich existing `EventTag::ActionAborted` and `EventTag::SleepEpisodeEnded` via payload widening, not parallel them)
- No CLI player-POV gating for `failed_rest_opportunities` (ticket 010)
- No removal of `ActionTraceDetail::SelfCareInterrupted` — that variant continues serving the other five self-care families per S173
- No `SAVE_FORMAT_VERSION` bump (rides ticket 001's bump via `#[serde(default)]`)
- No follow-up auto-creation of preempted-by-higher-need records if path (c) is deferred — leave a TODO marker referencing this ticket and document in Outcome

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit test: `FailedRestOpportunity` and `FailedRestKind` construct and pattern-match with all variants
2. New focused unit test: `CriticalWindowFrame.failed_rest_opportunities` round-trips through serde with default empty vec for pre-bump snapshots
3. New integration test: sleep abort mid-episode at a Place emits `ActionTraceDetail::SleepInterrupted { place: <id>, cause: <SleepFailureCause>, accumulated_recovery: <Permille>, was_rough_sleep: <bool> }`
4. New integration test: sleep abort mid-episode no longer emits `ActionTraceDetail::SelfCareInterrupted { kind: Sleep, ... }` (negative branch confirming the switch)
5. New integration test: during an active critical-fatigue window, a sleep abort appends `FailedRestOpportunity { kind: Interrupted { cause } }` to the window's frame
6. New integration test: during an active critical-fatigue window, a sleep action start failure (rest-site full) appends `FailedRestOpportunity { kind: PreconditionRejected }` to the window's frame
7. Existing suite: `cargo test -p worldwake-ai survival_forensics` passes
8. Existing suite: `cargo test --workspace` passes (regression risk: tests asserting `SelfCareInterrupted { kind: Sleep }` for sleep aborts need updates to assert `SleepInterrupted` instead)

### Invariants

1. Sleep aborts emit `ActionTraceDetail::SleepInterrupted` exclusively — never `SelfCareInterrupted { kind: Sleep }`
2. `FailedRestOpportunity` records are appended only during active critical-fatigue windows; out-of-window aborts do not generate records
3. `CriticalWindowFrame.failed_rest_opportunities` is `Vec<FailedRestOpportunity>` with `#[serde(default)]` so pre-bump serialized snapshots load with an empty vec

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/survival_forensics.rs` (extend inline `#[cfg(test)]`) — type derives, serde round-trip, three populating paths (paths (a) and (b) mandatory; path (c) deferred per Assumption 5)
2. `crates/worldwake-sim/src/tick_step.rs` (extend inline `#[cfg(test)]`) — `abort_trace_detail_for_instance` sleep-branch coverage with the new variant
3. Existing tests asserting `SelfCareInterrupted { kind: Sleep }` — update assertions to `SleepInterrupted` (locate via grep at ticket-implementation time)

### Commands

1. `cargo test -p worldwake-ai survival_forensics` (forensics coverage)
2. `cargo test -p worldwake-sim tick_step` (trace boundary coverage)
3. `cargo test --workspace` (full regression — sleep-abort test updates are the primary risk surface)
4. `./scripts/verify.sh` (final pre-PR gate)
