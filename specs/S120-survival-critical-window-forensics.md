# S120: Survival Critical-Window Forensics

## Summary

Add a stable, mechanical forensic surface for long-run survival failures so authored-critical windows can be explained without ad hoc ignored reproducers or one-off debug instrumentation. The new surface reports, for each prolonged critical window, the selected goal, top competing goals, active-action / exhaustion state, relevant planner provenance, and the local authoritative state needed to understand why the run stayed critical. This is a read-only traceability spec: it does not change survival behavior, only the causal report surface used when a scenario regresses.

## Phase and Status

Phase 7 Adjunct: Survival Stability Hardening. Status: Draft.

## Crates

- `worldwake-ai` — reusable survival-forensics helper over decision traces, action traces, and authoritative per-tick state
- `worldwake-cli` — optional observer/report rendering if the existing observer path is the best read-side home
- `worldwake-core` — no changes
- `worldwake-sim` — no changes
- `worldwake-systems` — no changes

## Dependencies

- None.
- Informs troubleshooting and maintenance of:
  - `golden_survival_baseline.rs`
  - `golden_survival_scattered.rs`
  - `golden_survival_contested.rs`
- Complements, but does not replace:
  - `S117` observer smell detection
  - `S118` stuck-detector precision

## Motivating Evidence

Implementation of the S116 survival ticket chain exposed a repeatable traceability gap:

1. After the stale exact-opportunity fix in ticket `011`, the remaining baseline failure was a long-run authored-critical window, not a single-step planner contradiction.
2. Existing decision traces were strong enough to prove single-tick selection facts, but not strong enough to explain a 100–400 tick survival window without adding temporary ignored reproducers and local probe code.
3. The eventual `Sleep` fix in ticket `012` required multiple rounds of ad hoc instrumentation just to distinguish stale-local opportunity bugs, ranking bias, threshold mismatch, and progress-barrier absence.

The gap is not that the system lacks traces entirely. The gap is that there is no stable, reusable read-model for "why did this need stay above authored critical for this long?"

## Design Goals

1. A prolonged authored-critical run can be explained through one deterministic forensic report rather than one-off debug helpers.
2. The report uses existing authoritative and planner-owned data; it does not add a second truth path.
3. The report is keyed to authored thresholds, not a hardcoded critical permille band.
4. The surface is reusable across survival baseline, scattered, and contested scenarios.
5. The report distinguishes at least these causal classes:
   - selected local self-care vs remote recovery competition
   - active action blocking / interruption context
   - `FrontierExhausted` / blocker suppression
   - local authoritative resource absence

## Non-Goals

- Changing AI ranking or planner behavior.
- Adding new event-log carriers.
- Creating a general-purpose observer replacement.
- Persisting forensic summaries in authoritative state.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-14 (World State Is Not Belief State) | The forensic view shows both planner-visible reasoning and authoritative local state explicitly, without conflating them. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | Critical-window reports are derived from traces and world snapshots; deleting them changes no world meaning. |
| FND-29 (Debuggability Is a Product Feature) | The entire spec exists to make long-run survival contradictions mechanically explainable. |
| FND-29A (Causal History Is Authoritative, Append-Only) | The report is computed from existing append-only traces and state snapshots, not a mutable side channel. |
| FND-31 (Validation and Falsification Are First-Class) | A failing survival window becomes a falsifiable causal report instead of a vague "scenario got unhealthy" symptom. |

## Deliverables

### D1: Critical-window report model

Add a derived report type in `worldwake-ai` test/support or shared observer code:

```rust
pub struct CriticalWindowReport {
    pub agent: EntityId,
    pub need: RankedDriveKind,
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub threshold: Permille,
    pub peak_value: Permille,
    pub frames: Vec<CriticalWindowFrame>,
}

pub struct CriticalWindowFrame {
    pub tick: Tick,
    pub need_value: Permille,
    pub selected_goal: Option<GoalKey>,
    pub selected_plan_source: Option<SelectedPlanSource>,
    pub top_competitors: Vec<RankedGoalSnapshot>,
    pub active_action: Option<String>,
    pub exhaustion_state: Option<String>,
    pub blocker_summary: Option<String>,
    pub local_authoritative_summary: LocalSurvivalStateSummary,
}
```

`RankedGoalSnapshot` is a bounded derived shape containing:

- `goal`
- `priority_class`
- `motive_score`
- compact provenance string (or typed subset) needed for survival comparison

### D2: Shared extractor for authored-critical windows

Add a helper that:

1. reads the agent's authored `DriveThresholds`
2. finds maximal runs where a need stays `>= authored critical`
3. returns the top-N longest windows or a specific requested window

This helper becomes the canonical way to investigate long-run survival failures in goldens.

### D3: Bounded frame capture rules

To keep reports legible and deterministic, capture:

- first 5 ticks of the window
- last 5 ticks of the window
- up to 5 evenly spaced interior ticks
- any tick where selected goal changes
- any tick where `selected_plan_source`, active action, blocker state, or exhaustion state changes

This avoids flooding while still showing causal changes.

### D4: Local authoritative survival summary

For each captured frame, include only the local authoritative state relevant to self-care diagnosis:

- actor effective place
- local water source presence / quantity
- local wash basin presence
- local sleep affordance presence if applicable
- local food source / edible stock presence if applicable

This summary exists to separate "planner failed despite local affordance" from "planner had no lawful local affordance and needed remote pursuit."

### D5: Golden/test integration

Add reusable assertion helpers so a failing survival golden can print a compact `CriticalWindowReport` directly from shared support instead of introducing bespoke ignored reproducers.

At minimum:

- one focused test proving a `Sleep` authored-critical fatigue window reports `FrontierExhausted` / progress-barrier context correctly
- one focused test proving a wash-vs-water competition window reports both competing goals and the selected winner

### D6: Optional observer/report rendering

If the existing observer binary is the best consumer, add a new optional section such as:

`Section 2.5 — Critical Window Forensics`

It renders only when requested, and only for the top-N longest authored-critical windows in the run.

Observer integration is optional for the first landing if the shared golden-support surface is sufficient, but the report model should be designed so observer reuse is straightforward.

### D7: Documentation

Update `docs/golden-e2e-testing.md` and/or `docs/debugging-traces.md` to point survival-debugging work at the shared critical-window report helper rather than ad hoc ignored reproducers.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Inputs are existing decision traces, action traces, authored `DriveThresholds`, and authoritative local state snapshots. No new simulation information path is introduced.
2. **Positive-feedback analysis**: None. The report is read-only and post hoc.
3. **Concrete dampeners**: Bounded frame capture is the report dampener; it limits output volume without hiding causal changes.
4. **Stored state vs. derived read-model**:
   - **Stored/authored**: none beyond existing traces and scenario thresholds
   - **Derived**: `CriticalWindowReport`, bounded frame summaries, local survival summaries

## SystemFn Integration

None. This spec adds no `SystemFn`.

## Component Registration

None.

## Cross-System Interactions

- `worldwake-ai` extracts planner/decision-side facts.
- Optional observer rendering reads the same derived report model.
- No simulation system depends on the report.

## Validation and Falsification

### Focused tests

1. A synthetic fatigue-critical window with repeated `Sleep` planning reports the correct authored threshold and `FrontierExhausted` / progress-barrier transition.
2. A synthetic thirst or dirtiness competition window reports both selected goal and top competitor with bounded provenance.
3. Bounded frame selection remains deterministic for the same trace input.

### Golden / integration tests

4. Survival baseline golden support can emit a stable `CriticalWindowReport` for the longest authored-critical run when requested.
5. A no-failure healthy run produces no critical-window report output unless explicitly asked for the top-N windows.

## Outcome

To be filled in at completion.
