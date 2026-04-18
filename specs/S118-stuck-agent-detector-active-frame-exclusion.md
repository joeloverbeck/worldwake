# S118: Stuck-Agent Detector Precision — Active-Frame Exclusion

## Summary

Refine the mechanical `STUCK_AGENT` observer anomaly detector to exclude tick windows where the agent was in an active multi-tick action frame (ActionStarted without a matching ActionCommitted/ActionAborted yet). The current detector counts "ticks with no ActionStarted event at tick T" as idle, which produces false positives during composite maintenance trips (travel → wash → travel) where the middle ticks of a 12-tick wash register as "no action started at this tick" even though the agent is demonstrably working. Observer-only change; no simulation behavior impact.

## Phase and Status

Phase 8 Adjunct: Survival Baseline Under Contention (post-`survival-contested.ron` report). Status: Draft.

## Crates

- `worldwake-cli` — observer binary at `crates/worldwake-cli/src/bin/observer.rs` (single-file binary; the `StuckAgent` detector is inline in `fn detect_anomalies` around lines 836-875, and the per-agent idle tracking lives in `AgentStats::record_idle_tick` at lines 149-195 with the call site at 3454).
- All other crates: no changes.

## Dependencies

- None on simulation side.
- S117 has already landed (archived at `archive/specs/S117-convergence-maintenance-observer-smells.md`). The observer-anomaly test file `crates/worldwake-cli/tests/golden_observer_anomalies.rs` and the fixture directory `crates/worldwake-cli/tests/fixtures/observer_anomalies/` exist and establish the canonical pattern this spec extends: scripted `.ron` fixture + `Command::new(env!("CARGO_BIN_EXE_observer"))` + text-report parsing via `count_anomalies_of_kind`/`anomaly_block`.

## Motivating Evidence

From `reports/scenario-analysis-report.md` Layer 3 § False Positives and the session skill-audit:

> The skill's Smell 3 note says *"the mechanical stuck-agent detector counts consecutive ticks with no action started or in-progress. Multi-tick actions like sleep occupy the agent and are not counted as idle."* In this session the detector flagged Agent C for 26 consecutive idle ticks (59-84) during what was demonstrably an active wash+travel cycle (wash = 12 ticks, plus 2×2-tick travel legs, plus harvest water). The agent's wash commit count confirms the action completed.

The skill (as of the S116/S117/S118 brainstorm) has already been patched to warn analysts about the detector's imprecision. This spec closes the loop by fixing the detector itself, so the warning language in the skill becomes redundant and is simplified back to "multi-tick actions are excluded".

## Design Goals

1. The `STUCK_AGENT` detector excludes tick windows that overlap with an open ActionStarted frame for that agent.
2. The refined detector still fires on genuinely stuck agents — those whose action trace contains no ActionStarted/ActionCommitted pairs across the window despite rising needs.
3. The detector's threshold (currently 20 consecutive ticks in the default observer, at `observer.rs:837`) is unchanged. This is a precision fix, not a sensitivity change. The independent 40-tick bound used by `crates/worldwake-ai/tests/golden_survival_contested.rs` comes from `scenarios/survival-contested.ron` (`max_idle_window_ticks_with_elevated_need: 40`) and is unrelated to the observer detector.
4. The fix is a local change inside the detector; no anomaly-format changes, no Section 3 rendering changes.

## Non-Goals

- Revising the idle threshold. Threshold calibration is a separate concern and may warrant its own work if follow-on runs reveal new pathologies.
- Extending the fix to other detectors. This spec touches the `StuckAgent` detector only.
- Exposing new observer CLI flags. The detector's behavior remains default-on and non-parameterized from the CLI.
- Synthesizing "pseudo-ActionStarted" frames for actions that trace differently (e.g., travel inside planner state). The fix uses only the existing `ActionTraceKind::Started` / `ActionTraceKind::Committed` / `ActionTraceKind::Aborted` lifecycle events already present in the action trace sink.
- Extracting the stuck-agent detector into a reusable library function. Tests reuse the S117 binary-invocation pattern; the detector stays inline in the observer binary.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-12 (Performance May Compress, Never Causality) | Observer read-only; no world-meaning change. |
| FND-26 (Systems Through State) | Detector reads the authoritative action trace; no writes. Tracking stays inside the single existing `AgentStats::record_idle_tick` code path rather than splitting idle state across two structures. |
| FND-29 (Debuggability Is a Product Feature) | A false-positive-prone detector reduces trust in the diagnostic suite. Precision is the product feature. |

## Deliverables

### D1: Feed open-frame state into the existing idle counter

Current code path in `crates/worldwake-cli/src/bin/observer.rs`:

- The outer scan loop already iterates per tick × per agent (lines 3378-3455).
- At lines 3450-3454 it computes:

  ```rust
  // Idle tracking: did this agent have any action trace events this tick?
  let had_action = action_trace
      .events_for_at(*agent_id, current_tick)
      .iter()
      .any(|e| !matches!(e.kind, ActionTraceKind::StartFailed { .. }));
  stats.record_idle_tick(had_action, current_tick.0, current_needs);
  ```

- `AgentStats::record_idle_tick(had_action, …)` at lines 149-195 is the single authoritative place where `consecutive_idle_ticks`, `max_consecutive_idle`, `idle_window_start`, and `idle_windows` are updated. `had_action == true` resets the idle window.

The fix extends this same `had_action` computation so that a tick spent inside an **open action frame** (agent has an unclosed `ActionStarted` from an earlier tick) is treated as non-idle. No parallel tracker, no second pre-scan, no new abstraction — the change is entirely local to the existing outer loop, keeping idle-tracking logic cohesive (FND-26, FND-29).

Proposed change (pseudocode; field name `event.actor` matches the real struct at `crates/worldwake-sim/src/action_trace.rs:23`):

```rust
// Declared once, just above the outer `for tick_num in 0..` loop:
// `true` while the agent has a Started event not yet matched by
// Committed/Aborted. StartFailed does NOT open a frame.
let mut open_frame: BTreeMap<EntityId, bool> = BTreeMap::new();

for tick_num in 0..total_ticks {
    let current_tick = Tick(u64::from(tick_num));
    // ... existing per-tick processing ...

    // Update open_frame from THIS tick's action trace events before
    // computing had_action. A Started+Committed on the same tick lands
    // in the Committed arm last and correctly leaves the frame closed.
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

    for (agent_id, stats) in &mut agent_stats {
        // ... existing needs/location sampling ...

        let had_event = action_trace
            .events_for_at(*agent_id, current_tick)
            .iter()
            .any(|e| !matches!(e.kind, ActionTraceKind::StartFailed { .. }));
        let in_open_frame = open_frame.get(agent_id).copied().unwrap_or(false);
        let had_action = had_event || in_open_frame;
        stats.record_idle_tick(had_action, current_tick.0, current_needs);
    }
}
```

Semantics:

- A tick with a Started, Committed, or Aborted event for the agent continues to count as non-idle (existing behavior preserved).
- A tick during which the agent has no trace event, but an earlier Started has not yet been matched by Committed/Aborted, also counts as non-idle (new behavior).
- StartFailed neither opens a frame nor (on its own) makes the tick non-idle — a failed start leaves the agent free to plan on the next tick, so the counter keeps climbing on subsequent empty ticks.
- Same-tick Started→Committed pairs are handled correctly because `open_frame` is updated from events at the current tick *before* the agent loop reads `had_action`: the Committed arm runs last and closes the frame before the agent loop observes it.

### D2: Test — false positive eliminated

Add a new fixture and a new test to the existing S117 harness, following the same fixture+binary+report-parse pattern already used by `convergence_smell_fires_on_forced_hub_scenario`, `maintenance_starvation_fires_on_wash_gap`, etc.

New fixture: `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_wash_travel_cycle.ron`. Design constraints:

- Exactly one agent positioned so its need-satisfaction cycle produces a composite trip analogous to the Agent C wash+travel window: at least one multi-tick maintenance action (e.g., `wash` at ~12 ticks) bracketed by travel legs, for a contiguous span longer than the observer's 20-tick default threshold and longer than the `refine_stuck_agents` low-need exemption (see `observer.rs:1930-1960`).
- Needs configured so the span does not count as "all needs low" — otherwise `refine_stuck_agents` would strip the anomaly regardless of the fix and the test would pass trivially without exercising D1.
- Total simulated ticks set just past the span so the test is cheap.

New test in `crates/worldwake-cli/tests/golden_observer_anomalies.rs`:

```rust
#[test]
fn stuck_detector_excludes_wash_travel_cycle() {
    let report = run_observer(
        "tests/fixtures/observer_anomalies/stuck_detector_wash_travel_cycle.ron",
        /* ticks: */ <span_length + small buffer>,
    );
    assert_eq!(
        count_anomalies_of_kind(&report, "STUCK_AGENT"),
        0,
        "wash+travel cycle must not register as stuck; Section 3:\n{}",
        section_three(&report),
    );
}
```

The test must fail against current `main` (where the detector counts middle wash ticks as idle) and pass after D1.

### D3: Test — genuine idle still fires

New fixture: `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_genuine_idle.ron`. Design:

- One agent with a control source or affordance configuration such that no ActionStarted events fire across a span comfortably exceeding the 20-tick threshold (e.g., a human-controlled agent receiving no input, or an AI agent whose available affordances are all gated so no goal becomes actionable).
- Needs configured so at least one need rises above the `refine_stuck_agents` low-need exemption during the span.

Test:

```rust
#[test]
fn stuck_detector_still_fires_on_genuine_idle() {
    let report = run_observer(
        "tests/fixtures/observer_anomalies/stuck_detector_genuine_idle.ron",
        /* ticks: */ <span_length + small buffer>,
    );
    assert_eq!(count_anomalies_of_kind(&report, "STUCK_AGENT"), 1);
    let block = anomaly_block(&report, "STUCK_AGENT");
    assert!(block.contains("consecutive ticks"));
}
```

### D4: Test — StartFailed does not open a frame

Implemented form after live reassessment: a focused unit test in `crates/worldwake-cli/src/bin/observer.rs`, not a pure observer-binary `.ron` fixture.

Why the seam changed:

- The live observer scenario schema does not expose an authored input/request surface that can truthfully produce a `StartFailed`-only span.
- Probe scenarios that attempted to force the shape through authored `.ron` setup produced lawful silence (`Harvest Water | 0`) rather than AI-driven `ActionTraceKind::StartFailed`.
- The honest missing proof surface was observer-local confirmation that `StartFailed` does **not** count as activity in the stuck-detector path.

Implemented proof shape:

- Create one synthetic agent stats entry with elevated needs.
- Record one `ActionTraceKind::StartFailed` event per tick in an `ActionTraceSink`.
- Recompute `had_event` with the same live observer filter (`!matches!(e.kind, ActionTraceKind::StartFailed { .. })`).
- Feed the resulting values through `AgentStats::record_idle_tick`, flush the final idle window, and assert `detect_anomalies(...)` still emits exactly one `AnomalyKind::StuckAgent`.

Test:

```rust
#[test]
fn stuck_detector_does_not_treat_startfailed_as_active_frame() {
    // synthetic StartFailed-only action trace must still accumulate idle
    // and survive the STUCK_AGENT emission/refinement path
}
```

### D5: Skill documentation touch-up (non-code)

After D1-D4 land, simplify `.claude/skills/scenario-analysis/references/layer-1-behavioral-smells.md` — the "Detector caveat" paragraph currently at line 21 contains the expanded warning ("behavior is not 100% reliable for composite maintenance trips", analyst-verification procedure, and the asymmetric-failure sentence) added by the 2026-04-17 skill-audit. With S118 live, that paragraph is replaced by the simpler form:

```
Detector caveat: the mechanical stuck-agent detector counts consecutive ticks
with no action *started or in-progress*. Multi-tick actions like sleep, wash,
and travel legs occupy the agent and are not counted as idle. Therefore
"max consecutive idle ticks" in Section 2 may exceed the detector's threshold
without triggering an anomaly.
```

Remove the "behavior is not 100% reliable for composite maintenance trips" clause, the "before classifying a flagged window as a false positive …" verification steps, and the trailing "anomalies may fire on windows containing active multi-tick work" sentence — all three are made obsolete by the D1 fix. `.claude/skills/scenario-analysis/SKILL.md` itself does not reference this clause and does not need editing.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Detector reads the authoritative action trace (`ActionTraceKind::Started/Committed/Aborted/StartFailed` events) exposed by `ActionTraceSink`. No new information paths, no simulation-state writes, no new perception or belief surfaces. The `open_frame` map is a per-run observer-local variable, not an ECS component or a persisted artifact.
2. **Positive-feedback analysis**: None. Observer is passive.
3. **Concrete dampeners**: Not applicable.
4. **Stored state vs. derived read-model**: The open-frame tracker is a transient per-run observer-local structure reconstructed from the authoritative action trace. Not persisted. Not authoritative. Simply a refinement of the existing anomaly derivation and fully replaceable by recomputation from the same trace.

## SystemFn Integration

None.

## Component Registration

None.

## Cross-System Interactions (FND-26)

None. Observer-only.

## Risks and Open Questions

1. **Interaction with S117 detectors**: S117 adds four new anomaly kinds that also read the action trace. Both specs should share a single action-trace scan pass for efficiency. Implementation concern, not correctness concern. S117 has already landed, so D1 will share the existing outer loop.
2. **Trace completeness**: The fix assumes every multi-tick action emits an ActionStarted at start and an ActionCommitted/Aborted at end. If a future action ever committed synchronously without a separate start event, it would open no frame and the detector would treat it as idle — which is correct (synchronous actions are single-tick). No known cases today violate this.
3. **Abort semantics**: If an action is aborted mid-frame (`ActionTraceKind::Aborted`), the frame closes on the abort tick. The agent's idle counter starts from the abort tick. Correct behavior — after an abort the agent is genuinely free to plan next tick.
4. **Fixture reproducibility**: D2-D4 rely on crafted `.ron` scenarios that must reliably induce the target action trace under deterministic replay. Small changes to default profiles or planner heuristics can shift when Started/Committed/StartFailed fire. If the fixtures prove brittle, narrow them to the smallest agent population and shortest time span that still exercises the detector behavior, and seed them explicitly.

## Verification Plan

1. `cargo test -p worldwake-cli --test golden_observer_anomalies` — D2 and D3 pass, and the existing S117 tests still pass.
2. `cargo test -p worldwake-cli --bin observer tests::stuck_detector_does_not_treat_startfailed_as_active_frame -- --exact` — D4 passes at the strongest honest observer-local seam.
3. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-contested.ron --ticks 1440 --output /tmp/contested-dump.md` — the historical Agent C 26-tick window (ticks 59-84) is no longer flagged as `STUCK_AGENT`.
4. `cargo test -p worldwake-ai --test golden_survival_contested` — still passes. This test is independent of the observer binary's detector: it runs its own per-agent idle tracker inside `no_stuck_idle_windows_with_elevated_needs` (see `golden_survival_contested.rs:593-602`) using scenario-authored thresholds read from the `SurvivalHealthContract` (`contract.max_idle_window_ticks_with_elevated_need = 40` and `contract.elevated_need_floor`, both sourced from `scenarios/survival-contested.ron:31`). S118 does not touch either the contract or the test's idle logic.
5. `cargo clippy --workspace --all-targets -- -D warnings` — clean.
