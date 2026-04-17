# S118: Stuck-Agent Detector Precision — Active-Frame Exclusion

## Summary

Refine the mechanical `STUCK_AGENT` observer anomaly detector to exclude tick windows where the agent was in an active multi-tick action frame (ActionStarted without a matching ActionCommitted/ActionAborted yet). The current detector counts "ticks with no ActionStarted event at tick T" as idle, which produces false positives during composite maintenance trips (travel → wash → travel) where the middle ticks of a 12-tick wash register as "no action started at this tick" even though the agent is demonstrably working. Observer-only change; no simulation behavior impact.

## Phase and Status

Phase 8 Adjunct: Survival Baseline Under Contention (post-`survival-contested.ron` report). Status: Draft.

## Crates

- `worldwake-cli` — observer binary (`src/observer/anomalies.rs` or equivalent): revise the `StuckAgent` detector
- All other crates: no changes

## Dependencies

- None on simulation side.
- S117 references this refined behavior in its ordering/false-positive notes but does not depend on it for landing.

## Motivating Evidence

From `reports/scenario-analysis-report.md` Layer 3 § False Positives and the session skill-audit:

> The skill's Smell 3 note says *"the mechanical stuck-agent detector counts consecutive ticks with no action started or in-progress. Multi-tick actions like sleep occupy the agent and are not counted as idle."* In this session the detector flagged Agent C for 26 consecutive idle ticks (59-84) during what was demonstrably an active wash+travel cycle (wash = 12 ticks, plus 2×2-tick travel legs, plus harvest water). The agent's wash commit count confirms the action completed.

The skill (as of the S116/S117/S118 brainstorm) has already been patched to warn analysts about the detector's imprecision. This spec closes the loop by fixing the detector itself, so the warning language in the skill becomes redundant and is simplified back to "multi-tick actions are excluded".

## Design Goals

1. The `STUCK_AGENT` detector excludes tick windows that overlap with an open ActionStarted frame for that agent.
2. The refined detector still fires on genuinely stuck agents — those whose action trace contains no ActionStarted/ActionCommitted pairs across the window despite rising needs.
3. The detector's threshold (currently 20 consecutive ticks in default observer; 40 in `golden_survival_contested.rs::IDLE_THRESHOLD`) is unchanged. This is a precision fix, not a sensitivity change.
4. The fix is a local change inside the detector; no anomaly-format changes, no Section 3 rendering changes.

## Non-Goals

- Revising the idle threshold. Threshold calibration is a separate concern and may warrant its own work if follow-on runs reveal new pathologies.
- Extending the fix to other detectors. This spec touches the `StuckAgent` detector only.
- Exposing new observer CLI flags. The detector's behavior remains default-on and non-parameterized from the CLI.
- Synthesizing "pseudo-ActionStarted" frames for actions that trace differently (e.g., travel inside planner state). The fix uses only the existing `ActionTraceKind::Started` / `ActionTraceKind::Committed` / `ActionTraceKind::Aborted` lifecycle events already present in the action trace sink.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-12 (Performance May Compress, Never Causality) | Observer read-only; no world-meaning change. |
| FND-26 (Systems Through State) | Detector reads the authoritative action trace; no writes. |
| FND-29 (Debuggability Is a Product Feature) | A false-positive-prone detector reduces trust in the diagnostic suite. Precision is the product feature. |

## Deliverables

### D1: Active-frame tracker during detection

In the existing `StuckAgent` detector loop (observer binary):

```rust
// Before the tick scan, build a per-agent open-frame tracker:
// open_frame[agent] = Some(start_tick) when agent has an ActionStarted
// without a matching ActionCommitted/ActionAborted.
let mut open_frame: BTreeMap<EntityId, Option<Tick>> = BTreeMap::new();

for tick in 0..max_tick {
    for event in action_trace.events_at(tick) {
        match event.kind {
            ActionTraceKind::Started { .. } => {
                open_frame.insert(event.agent, Some(tick));
            }
            ActionTraceKind::Committed { .. } | ActionTraceKind::Aborted { .. } => {
                open_frame.insert(event.agent, None);
            }
            ActionTraceKind::StartFailed { .. } => {
                // StartFailed does not open a frame.
            }
        }
    }

    for (agent, frame) in &open_frame {
        let is_in_active_frame = frame.is_some();
        // Only count ticks toward idle when NOT in an active frame.
        idle_tracker.observe(*agent, tick, is_in_active_frame);
    }
}
```

The idle counter resets when `is_in_active_frame == true`, just as it resets on an ActionStarted.

### D2: Test — false positive eliminated

New test in `crates/worldwake-cli/tests/golden_observer_anomalies.rs` (or whichever file hosts the observer tests, reusing the file created by S117 if that has landed):

```rust
#[test]
fn stuck_detector_excludes_wash_travel_cycle() {
    // Scripted trace:
    // tick 0: Agent A — ActionStarted { action: "travel" }
    // tick 2: Agent A — ActionCommitted { action: "travel" }
    // tick 3: Agent A — ActionStarted { action: "harvest:Harvest Water" }
    // tick 11: Agent A — ActionCommitted { action: "harvest:Harvest Water" }
    // tick 12: Agent A — ActionStarted { action: "wash" }
    // tick 24: Agent A — ActionCommitted { action: "wash" }
    // Full span: 25 ticks, no idle gap > default threshold.

    let anomalies = detect_stuck_agents(&scripted_trace, &needs_trajectory);
    assert!(anomalies.is_empty(), "expected zero STUCK_AGENT anomalies; got {anomalies:?}");
}
```

### D3: Test — genuine idle still fires

Same file:

```rust
#[test]
fn stuck_detector_still_fires_on_genuine_idle() {
    // Scripted trace:
    // tick 0: Agent A — (no events)
    // ...
    // tick 50: Agent A — (no events)
    // Needs rising: hunger 400 -> 600 across the span.

    let anomalies = detect_stuck_agents(&scripted_trace, &needs_trajectory);
    assert_eq!(anomalies.len(), 1);
    assert!(matches!(anomalies[0].kind, AnomalyKind::StuckAgent { .. }));
}
```

### D4: Test — StartFailed does not open a frame

```rust
#[test]
fn stuck_detector_does_not_treat_startfailed_as_active_frame() {
    // Scripted trace:
    // tick 0: Agent A — ActionTraceKind::StartFailed { action: "wash" }
    // ticks 1..50: (no events) + needs rising
    // The StartFailed must NOT be treated as an open frame — the agent
    // is genuinely idle across ticks 1..50.

    let anomalies = detect_stuck_agents(&scripted_trace, &needs_trajectory);
    assert_eq!(anomalies.len(), 1);
}
```

### D5: Skill documentation touch-up (non-code)

After D1-D4 land, revert the `.claude/skills/scenario-analysis/SKILL.md` Smell 3 note added by the 2026-04-17 skill-audit back to its simpler form:

```
Note: the mechanical stuck-agent detector counts consecutive ticks with no
action *started or in-progress*. Multi-tick actions like sleep, wash, and
travel legs occupy the agent and are not counted as idle. Therefore
"max consecutive idle ticks" in Section 2 may exceed the detector's threshold
without triggering an anomaly.
```

Remove the added "detector behavior is not 100% reliable" clause, since S118 makes the note true in practice.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Detector reads the authoritative action trace (`ActionTraceKind::Started/Committed/Aborted/StartFailed` events). No new information paths, no simulation-state writes.
2. **Positive-feedback analysis**: None. Observer is passive.
3. **Concrete dampeners**: Not applicable.
4. **Stored state vs. derived read-model**: The open-frame tracker is a transient per-run observer-local structure. Not persisted. Not authoritative. Simply a refinement of the existing anomaly derivation.

## SystemFn Integration

None.

## Component Registration

None.

## Cross-System Interactions (FND-26)

None. Observer-only.

## Risks and Open Questions

1. **Interaction with S117 detectors**: S117 adds four new anomaly kinds that also read the action trace. Both specs should share a single action-trace scan pass for efficiency. Implementation concern, not correctness concern.
2. **Trace completeness**: The fix assumes every multi-tick action emits an ActionStarted at start and an ActionCommitted/Aborted at end. If a future action ever committed synchronously without a separate start event, it would open no frame and the detector would treat it as idle — which is correct (synchronous actions are single-tick). No known cases today violate this.
3. **Abort semantics**: If an action is aborted mid-frame (`ActionTraceKind::Aborted`), the frame closes on the abort tick. The agent's idle counter starts from the abort tick. Correct behavior — after an abort the agent is genuinely free to plan next tick.

## Verification Plan

1. `cargo test -p worldwake-cli --test golden_observer_anomalies` — all three new tests pass (D2, D3, D4)
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-contested.ron --ticks 1440 --output /tmp/contested-dump.md` — the historical Agent C 26-tick window is no longer flagged
3. `cargo test -p worldwake-ai --test golden_survival_contested` — still passes (this test uses its own `IDLE_THRESHOLD=40` / `NEEDS_LOW_CEILING` logic in-test, independent of the observer binary's detector, so S118 does not change its behavior)
4. `cargo clippy --workspace --all-targets -- -D warnings` — clean
