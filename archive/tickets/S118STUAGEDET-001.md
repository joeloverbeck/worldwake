# S118STUAGEDET-001: Active-frame tracker in stuck-agent detector

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — observer binary only; no simulation state, no action definitions, no belief paths touched.
**Deps**: specs/S118-stuck-agent-detector-active-frame-exclusion.md

## Problem

The mechanical `STUCK_AGENT` observer anomaly detector produces false positives during composite multi-tick actions (travel → wash → travel, harvest → consume, etc.). Current code at `crates/worldwake-cli/src/bin/observer.rs:3450-3453` treats a tick as idle when no action-trace event fires at that exact tick — but the middle ticks of a 12-tick wash have no events (Started fires at tick N, Committed fires at tick N+11, ticks N+1..N+10 are event-free). The detector counted the Agent C 26-tick wash+travel window from `reports/scenario-analysis-report.md` as stuck despite demonstrated active work. Precision in diagnostics is a first-class concern (FND-29); a noisy detector undermines trust in the observer suite and forces analysts to hand-verify every STUCK_AGENT flag against Section 4 action-lifecycle pairs.

## Assumption Reassessment (2026-04-18)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Current code state confirmed: `fn detect_anomalies` at `crates/worldwake-cli/src/bin/observer.rs:786-969` contains the StuckAgent detection block at lines 836-875. Idle tracking is owned by `AgentStats::record_idle_tick` at lines 149-195 with the sole call site at line 3454, reached from the outer scan loop at lines 3378-3455. `had_action` is computed at lines 3450-3453 by inspecting `action_trace.events_for_at(*agent_id, current_tick)` and filtering out `ActionTraceKind::StartFailed`. `ActionTraceEvent` in `crates/worldwake-sim/src/action_trace.rs:20-28` exposes `actor: EntityId` (not `agent`). The observer binary has 58 inline `#[test]` fns under `#[cfg(test)]` starting at line 3673, none of which exercise `record_idle_tick` or stuck-agent detection — this ticket introduces first-time coverage. Existing binary-invocation tests in `crates/worldwake-cli/tests/golden_observer_anomalies.rs` already use the `run_observer(…) → count_anomalies_of_kind(&report, …)` pattern with fixtures in `tests/fixtures/observer_anomalies/`; S117's landing made this the canonical harness.
2. Spec reference: `specs/S118-stuck-agent-detector-active-frame-exclusion.md` D1 (detector runtime fix) and D2 (wash+travel regression test). The spec was reassessed on 2026-04-18 and rewritten to match the existing `record_idle_tick` API and S117 binary-invocation test pattern.
3. Shared boundary under audit: `AgentStats::record_idle_tick(had_action, current_tick, needs)` — the single authoritative idle-tracking entry point. The change extends the `had_action` input *upstream* of this call; the call itself and `AgentStats` fields (`consecutive_idle_ticks`, `max_consecutive_idle`, `idle_window_start`, `idle_windows`) are untouched so the downstream detector logic at lines 836-875 and `refine_stuck_agents` at lines 1930-1960 remain the authoritative filter/reporting pipeline.

## Architecture Check

1. **Single source of truth preserved (FND-26)**: The `open_frame: BTreeMap<EntityId, bool>` introduced by this ticket is a transient per-run outer-loop local used only to compute the `had_action` input to the existing `record_idle_tick` call. It is not a parallel idle tracker — there is still exactly one code path that updates `AgentStats::consecutive_idle_ticks` and friends. Had the fix introduced a separate `idle_tracker.observe(...)` abstraction (the spec's original pseudocode), idle state would have been split across two structures, violating FND-26.
2. **Debuggability is a product feature (FND-29)**: Reducing false positives raises the signal-to-noise ratio of the anomaly report. Analysts no longer have to cross-reference every flagged window against Section 4 ActionStarted/ActionCommitted pairs to decide whether the anomaly is real.
3. **No backwards-compat shims (FND-28)**: The old `had_action` semantics are replaced in-place. No flag, no alias path, no "legacy detector mode."

## Verification Layers

1. Detector precision (no false-positive STUCK_AGENT on a composite multi-tick trip) -> observer binary invocation + text report parse via `count_anomalies_of_kind(&report, "STUCK_AGENT") == 0`.
2. Same-tick Started+Committed pairs still close the frame correctly -> fixture design ensures the `open_frame` update order (events iterated in `sequence_in_tick` order before the per-agent inner loop reads the flag) yields `in_open_frame == false` for the following tick; verified indirectly by the absence of spurious non-idle windows on pathological fixtures in later tickets.
3. Single-layer ticket: observer is read-only over the authoritative action trace; no additional simulation-layer invariant needs a distinct proof surface.

## What to Change

### 1. Declare per-run open-frame tracker

In `crates/worldwake-cli/src/bin/observer.rs`, immediately before the outer `for tick_num in 0..cli.ticks` loop (which begins around line 3378), declare:

```rust
// Per-agent open-frame tracker: `true` while the agent has an
// ActionStarted event not yet matched by Committed/Aborted.
// StartFailed does NOT open a frame.
let mut open_frame: BTreeMap<EntityId, bool> = BTreeMap::new();
```

### 2. Update open-frame state from this tick's action-trace events

At the top of the outer loop body (before the existing per-agent inner loop that begins at line 3419), add:

```rust
for event in action_trace.events_at(current_tick) {
    match &event.kind {
        ActionTraceKind::Started { .. } => {
            open_frame.insert(event.actor, true);
        }
        ActionTraceKind::Committed { .. } | ActionTraceKind::Aborted { .. } => {
            open_frame.insert(event.actor, false);
        }
        ActionTraceKind::StartFailed { .. } => {
            // StartFailed does not open a frame.
        }
    }
}
```

`events_at` already returns events sorted by `sequence_in_tick`, so a same-tick Started+Committed pair lands Started → Committed in order and the Committed arm correctly leaves `open_frame` closed before the per-agent inner loop reads it.

### 3. Fold open-frame state into `had_action`

Replace the existing `had_action` computation at lines 3450-3453 with:

```rust
let had_event = action_trace
    .events_for_at(*agent_id, current_tick)
    .iter()
    .any(|e| !matches!(e.kind, ActionTraceKind::StartFailed { .. }));
let in_open_frame = open_frame.get(agent_id).copied().unwrap_or(false);
let had_action = had_event || in_open_frame;
stats.record_idle_tick(had_action, current_tick.0, current_needs);
```

### 4. Regression fixture

Create `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_wash_travel_cycle.ron`. Design constraints:

- Exactly one AI-controlled agent.
- Place topology and facility tags arranged so the agent's need-satisfaction cycle produces a contiguous multi-tick action span longer than 20 ticks (the default `STUCK_AGENT` threshold at `observer.rs:837`). A wash (≈12 ticks) bracketed by travel legs is the canonical shape — match the motivating Agent C window.
- At least one need configured to rise above the 300-permille `NEEDS_LOW_CEILING` used by `refine_stuck_agents` at `observer.rs:1931-1960` during the span, so the window is not stripped by the low-need refinement filter regardless of this ticket's fix.
- Simulated tick budget set just past the span to keep the test cheap.

Cross-reference the existing fixture conventions in `crates/worldwake-cli/tests/fixtures/observer_anomalies/maintenance_starvation_wash_gap.ron` for `AgentDef` layout and topology syntax.

### 5. Regression test

Add to `crates/worldwake-cli/tests/golden_observer_anomalies.rs`:

```rust
#[test]
fn stuck_detector_excludes_wash_travel_cycle() {
    let report = run_observer(
        "tests/fixtures/observer_anomalies/stuck_detector_wash_travel_cycle.ron",
        /* ticks: span length + small buffer */,
    );
    assert_eq!(
        count_anomalies_of_kind(&report, "STUCK_AGENT"),
        0,
        "wash+travel cycle must not register as stuck; Section 3:\n{}",
        section_three(&report),
    );
}
```

The test must fail against `main` (the current detector flags the middle wash ticks as idle) and pass after changes 1-3 land. Running it before change 1-3 is the regression proof.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — declaration near outer loop, per-tick open-frame update, `had_action` extension)
- `crates/worldwake-cli/tests/golden_observer_anomalies.rs` (modify — append new test)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_wash_travel_cycle.ron` (new)

## Out of Scope

- Changing either the default 20-tick observer threshold or the scenario-authored `max_idle_window_ticks_with_elevated_need` (spec Non-Goal: threshold calibration is separate).
- Extending the fix to other detectors (spec Non-Goal: StuckAgent only).
- Exposing new CLI flags (spec Non-Goal).
- Synthesizing pseudo-ActionStarted frames for actions that trace differently (spec Non-Goal).
- Extracting the stuck-agent detector into a library function (spec Non-Goal — tests reuse the S117 binary-invocation pattern instead).
- The two remaining fixtures and tests (`stuck_detector_genuine_idle.ron`, `stuck_detector_startfailed_idle.ron`) — delivered by S118STUAGEDET-002.
- Skill documentation simplification — delivered by S118STUAGEDET-003.

## Acceptance Criteria

### Tests That Must Pass

1. `stuck_detector_excludes_wash_travel_cycle` — new test; must fail before changes 1-3 and pass after, proving the runtime fix.
2. All four existing tests in `golden_observer_anomalies.rs` (`convergence_smell_fires_on_forced_hub_scenario`, `convergence_smell_stays_absent_on_survival_baseline`, `maintenance_starvation_fires_on_wash_gap`, `recipe_monoculture_fires_on_single_food_dependency`, `acute_need_spike_fires_on_bounded_thirst_run`) — must continue passing; the `had_action` extension must not regress the counts any of them assert.
3. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. `had_action == true` on every tick that falls inside an open `ActionStarted` → `ActionCommitted`/`Aborted` frame for the agent, even when no trace event fires at that specific tick.
2. `ActionTraceKind::StartFailed` does not open a frame — an agent whose only recent event is a StartFailed, with no subsequent Started, continues to accumulate idle ticks on empty-event ticks.
3. `AgentStats::record_idle_tick` remains the single authoritative updater of `consecutive_idle_ticks`, `max_consecutive_idle`, `idle_window_start`, and `idle_windows`. No parallel idle-state structure is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/tests/golden_observer_anomalies.rs` — append `stuck_detector_excludes_wash_travel_cycle`; proves the false-positive is eliminated.
2. `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_wash_travel_cycle.ron` — new fixture crafted to reproduce the motivating Agent C composite-trip span while clearing the 300-permille refinement filter.

### Commands

1. `cargo test -p worldwake-cli --test golden_observer_anomalies stuck_detector_excludes_wash_travel_cycle` — targeted proof that the new test passes post-fix.
2. `cargo test -p worldwake-cli --test golden_observer_anomalies` — full observer-anomaly suite, ensuring S117's detectors still fire correctly.
3. `cargo test -p worldwake-cli` — crate-wide regression guard (includes `integration.rs` and the inline `#[cfg(test)]` block).
4. `cargo clippy --workspace --all-targets -- -D warnings` — lint parity with CI.

## Outcome

Completed on 2026-04-18.

- Extended the observer's existing idle-tracking path in `crates/worldwake-cli/src/bin/observer.rs` with a per-run `open_frame: BTreeMap<EntityId, bool>`, updated from same-tick action-trace lifecycle events before the per-agent idle sampling loop.
- Kept `AgentStats::record_idle_tick` as the single authoritative updater of idle windows; the only behavioral change is that `had_action` now resolves to `had_event || in_open_frame`.
- Added `stuck_detector_excludes_wash_travel_cycle` to `crates/worldwake-cli/tests/golden_observer_anomalies.rs` and a new distilled fixture at `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_wash_travel_cycle.ron` that reproduces the pre-fix false positive on a long active wash frame.

## Deviations

- The landed regression fixture is a distilled long-wash active-frame repro with a 30-tick observer run, not a full post-wash travel completion loop. Reassessment showed the draft's remote-facility/travel narrative pulled in unrelated belief-discovery and post-action idle noise, while the active-frame bug is already proved honestly by the shipped fixture's 22-tick mid-wash false positive on the pre-fix detector.

## Verification Result

- Passed `cargo test -p worldwake-cli --test golden_observer_anomalies stuck_detector_excludes_wash_travel_cycle -- --exact`
- Passed `cargo test -p worldwake-cli --test golden_observer_anomalies`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
