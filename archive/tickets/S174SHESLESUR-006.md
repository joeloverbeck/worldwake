# S174SHESLESUR-006: Failed-rest forensics — FailedRestOpportunity records + ActionTraceDetail::SleepInterrupted population

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `FailedRestOpportunity` and `FailedRestKind` types in `worldwake-ai/src/survival_forensics.rs`; new `failed_rest_opportunities` field on `CriticalWindowFrame`; population of `ActionTraceDetail::SleepInterrupted` at the sleep-abort trace boundary
**Deps**: `archive/tickets/S174SHESLESUR-001.md` (SleepFailureCause enum, ActionTraceDetail::SleepInterrupted variant), `archive/tickets/S174SHESLESUR-003.md` (belief-view accessors for failure attribution at candidate-rejection sites), `archive/tickets/S174SHESLESUR-004.md` (abort-cause mapping + ActionState::Sleep mode carrier)

## Problem

S174 D8 required `SurvivalForensicExtractor` to capture failed-rest opportunities during active critical fatigue windows so a future reader can answer "this agent collapsed from exhaustion because it failed to rest N times for these specific reasons." Before this ticket, `CriticalWindowFrame` (`crates/worldwake-ai/src/survival_forensics.rs:30-40`) captured exhaustion state and blocker summary but had no typed surface for failed-rest events. Additionally, ticket 001 added the `ActionTraceDetail::SleepInterrupted` variant; this ticket populated it at the abort-trace boundary in `tick_step.rs::abort_trace_detail_for_instance`, redirecting sleep aborts from the existing `SelfCareInterrupted { kind: Sleep, ... }` path.

The paired spec S175 reads `CriticalWindowReport.failed_rest_opportunities` to prove "fatigue collapse follows N failed-rest opportunities."

## Assumption Reassessment (2026-05-26)

1. Verified pre-implementation code: `SurvivalForensicExtractor` at `crates/worldwake-ai/src/survival_forensics.rs:157-161` aggregated active_windows + completed_reports. `CriticalWindowFrame` at lines 30-40 carried `tick, need_value, selected_goal, selected_plan_source, top_competitors, active_action, exhaustion_state, blocker_summary, local_authoritative_summary`. `CriticalWindowReport` at lines 19-27 carried `agent, need, start_tick, end_tick, threshold, peak_value, frames`. Both derived `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `abort_trace_detail_for_instance` at `crates/worldwake-sim/src/tick_step.rs:68-100` was the trace-emission boundary that mapped abort events to `ActionTraceDetail` variants; before this ticket it emitted `SelfCareInterrupted { kind: Sleep, basin: None }` for sleep aborts.
2. Spec assumption verified against S174 D7 and D8. The Implementation choice in D7 ("the abort helper emits `SleepInterrupted` for sleep actions and `SelfCareInterrupted` for the other five families") is a redirection — this ticket implements that switch.
3. Shared abstraction boundary under audit: forensic record schema (`CriticalWindowFrame` field extension) + action-trace boundary (`abort_trace_detail_for_instance` variant routing). The forensic recorder reads from the action-trace + event-log streams to construct `FailedRestOpportunity` entries; the action-trace boundary feeds the structured cause via the new `SleepInterrupted` variant.
4. The `failed_rest_opportunities: Vec<FailedRestOpportunity>` field on `CriticalWindowFrame` needs `#[serde(default)]` so existing serialized critical-window snapshots continue to deserialize (rides ticket 001's `SAVE_FORMAT_VERSION 107→108` bump without bumping again).
5. Three populating paths were landed: (a) sleep aborts mid-episode → write `FailedRestOpportunity { kind: Interrupted { cause }, ... }` from the abort trace detail; (b) sleep action start fails the rest-site precondition → write `FailedRestOpportunity { kind: PreconditionRejected, ... }` from the `StartFailed` action trace path; (c) actor abandons a Sleep intention for another need during a critical window → write `FailedRestOpportunity { kind: PreemptedByHigherNeed { need }, ... }` from the decision-trace goal-switch path.
6. Mismatch + correction: the existing `SelfCareInterrupted { kind: Sleep, basin: None }` path stays usable for trace-sink consumers that do not care about the structured cause. The new `SleepInterrupted` variant carries strictly more information (place, cause, accumulated_recovery, was_rough_sleep). The abort helper picks `SleepInterrupted` for sleep aborts; existing tests asserting `SelfCareInterrupted { kind: Sleep, ... }` need updates to assert `SleepInterrupted { ... }` instead.

## Architecture Check

1. Two new types (`FailedRestOpportunity`, `FailedRestKind`) are introduced in `worldwake-ai/src/survival_forensics.rs` — the same module that owns `CriticalWindowFrame`. Locating the types here (rather than in `worldwake-sim` or `worldwake-core`) keeps the forensic schema crate-private to `worldwake-ai`. The types are derived per-decision records (FND-27 derived view), not authoritative state — they aggregate event-log + action-trace data into a forensic snapshot.
2. Routing sleep aborts to `ActionTraceDetail::SleepInterrupted` (instead of the existing `SelfCareInterrupted { kind: Sleep }`) preserves FND-28 — one structured cause surface, not two. The `SelfCareInterrupted { kind: Sleep }` path is removed for the sleep-abort case; `SelfCareInterrupted` continues to handle the other five self-care families per S173.
3. The forensic record reads from authoritative event-log + trace streams (FND-26: systems via state); the recorder does not subscribe to the planner directly. This keeps the failure-attribution discipline decoupled from planning logic.

## Verified Layers

1. `FailedRestOpportunity` and `FailedRestKind` compile with `Clone`, `Debug`, `PartialEq`, `Eq`, `Serialize`, and `Deserialize` derives and focused variant coverage.
2. `CriticalWindowFrame.failed_rest_opportunities` has `#[serde(default)]`; a missing-field serde fixture loads with an empty vector.
3. Sleep abort trace detail now emits `ActionTraceDetail::SleepInterrupted { place, cause, accumulated_recovery, was_rough_sleep }`.
4. The sleep abort trace reads place, cause, and accumulated recovery from the append-only `SleepEpisodeEnded` event payload, while rough/known mode comes from `ActionState::Sleep`.
5. Non-sleep self-care aborts still emit `SelfCareInterrupted`.
6. Active fatigue-critical windows record `Interrupted`, `PreconditionRejected`, and `PreemptedByHigherNeed` failed-rest opportunities.
7. Failed-rest opportunities are not recorded outside active fatigue-critical windows.

## Landed Changes

1. Added `FailedRestOpportunity` and `FailedRestKind` in `crates/worldwake-ai/src/survival_forensics.rs`.
2. Added `CriticalWindowFrame.failed_rest_opportunities: Vec<FailedRestOpportunity>` with `#[serde(default)]`.
3. Extended survival-forensic frame building to derive failed-rest rows from action trace events and decision-trace goal switches only while the active critical window is Fatigue.
4. Updated `crates/worldwake-sim/src/tick_step.rs::abort_trace_detail_for_instance` so sleep aborts use `SleepInterrupted` and other self-care families keep `SelfCareInterrupted`.
5. Updated the S173 self-care interruption golden to expect `SleepInterrupted` for sleep and `SelfCareInterrupted` for the other self-care families.
6. Updated the CLI observer critical-window test fixture to populate the new frame field.

## Landed Files

- `crates/worldwake-ai/src/survival_forensics.rs`
- `crates/worldwake-sim/src/tick_step.rs`
- `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption.rs`
- `crates/worldwake-cli/src/bin/observer.rs`

## Out of Scope

- No new `EventTag` variant (FND-28: enrich existing `EventTag::ActionAborted` and `EventTag::SleepEpisodeEnded` via payload widening, not parallel them)
- No CLI player-POV gating for `failed_rest_opportunities` (ticket 010)
- No removal of `ActionTraceDetail::SelfCareInterrupted` — that variant continues serving the other five self-care families per S173
- No `SAVE_FORMAT_VERSION` bump (rides ticket 001's bump via `#[serde(default)]`)
- No follow-up for preempted-by-higher-need records; the decision-trace goal-switch path landed in this ticket.

## Acceptance Result

1. Passed: focused forensics tests cover `FailedRestOpportunity` and all `FailedRestKind` variants.
2. Passed: focused serde test covers missing `failed_rest_opportunities` defaulting to empty.
3. Passed: focused tick-step tests cover sleep abort detail construction from fallback state and from `SleepEpisodeEnded` event payload.
4. Passed: S173 golden now proves sleep uses `SleepInterrupted`; non-sleep families still use `SelfCareInterrupted`.
5. Passed: focused forensics tests cover interrupted sleep, rest-site start rejection, higher-need preemption, and out-of-window non-recording.
6. Passed: workspace regression caught and fixed the CLI observer fixture constructor for the new frame field.

## Test Plan Result

1. Added `survival_forensics.rs` tests for the new record types, serde defaulting, and all three failed-rest population paths.
2. Added `tick_step.rs` tests for structured sleep abort trace detail.
3. Updated `survival_self_care_interruption.rs` golden expectations for the sleep-specific trace detail.
4. Updated the CLI observer test fixture with an empty `failed_rest_opportunities` list.

## Outcome

Completed on 2026-05-26.

S174 now has a typed failed-rest forensic carrier. Active fatigue-critical windows can explain interrupted sleep, rest-site start rejection, and Sleep preemption by a higher-priority homeostatic need. Sleep abort action traces now use the structured `SleepInterrupted` detail populated from the event log and action state.

## Deviations

1. The drafted start-failure hook described subscribing to `EventTag::ActionAborted`; start failures are represented by `ActionTraceKind::StartFailed`, not an abort event, so the landed path records `PreconditionRejected` from sleep start-failure trace events whose reason identifies a full rest site.
2. The drafted preemption path was marked possibly deferred; it landed in this ticket by reading the existing decision-trace `goal_switch` from Sleep to another homeostatic goal.
3. The drafted `./scripts/verify.sh` row was not run as a wrapper during this per-ticket iteration. Its live subcommands were run directly; the implement-spec-tickets final branch phase still owns the full pre-PR wrapper gate before push.

## Verification Result

- Passed `cargo test -p worldwake-ai survival_forensics`
- Passed `cargo test -p worldwake-sim tick_step`
- Passed `cargo test -p worldwake-ai --test golden_ai golden_self_care_abort_traces_cover_every_family`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test --workspace`
- Passed `cargo fmt --all -- --check`
- Passed `cargo clippy --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Waived `./scripts/verify.sh` wrapper because this per-ticket iteration ran every live wrapper subcommand directly and the final implement-spec-tickets branch phase still owns the full pre-PR wrapper gate before push.
