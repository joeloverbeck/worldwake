# S120: Survival Critical-Window Forensics

## Summary

Add a stable, mechanical forensic surface for long-run survival failures so authored-critical windows can be explained without ad hoc ignored reproducers or one-off debug instrumentation. The new surface reports, for each prolonged critical window, the selected goal, top competing goals, active-action / exhaustion state, relevant planner provenance, and the local authoritative state needed to understand why the run stayed critical. This is a read-only traceability spec: it does not change survival behavior, only the causal report surface used when a scenario regresses.

## Phase and Status

Phase 7 Adjunct: Survival Stability Hardening. Status: ✅ COMPLETED.

## Crates

- `worldwake-ai` — new runtime module `crates/worldwake-ai/src/survival_forensics.rs` holding the report model and the per-tick window extractor; re-exported from `worldwake-ai/src/lib.rs`. Test-facing assertion wrappers stay under `crates/worldwake-ai/tests/golden_harness/mod.rs` and compose over the runtime types.
- `worldwake-cli` — optional observer/report rendering (D6) consuming the runtime types from `worldwake-ai`
- `worldwake-core` — no changes
- `worldwake-sim` — no changes
- `worldwake-systems` — no changes

## Dependencies

- None. S120 has no hard dependencies. Its window detector reads authored `DriveThresholds` and per-tick authoritative physiology state directly; it does not consume output from any other spec's detector.
- Informs troubleshooting and maintenance of:
  - `crates/worldwake-ai/tests/golden_survival_baseline.rs`
  - `crates/worldwake-ai/tests/golden_survival_scattered.rs`
  - `crates/worldwake-ai/tests/golden_survival_contested.rs`
- Complements, but does not replace and does not depend on:
  - `S117` observer smell detection — S117's `AcuteNeedSpike` (30–99 tick sub-threshold runs) and `MaintenanceStarvation` (relief-rate deficit over 200-tick windows) define different detection surfaces than S120's "need ≥ authored critical for N ticks" windows. An analyst can cross-reference an S117 anomaly with an S120 report, but S120 runs independently.
  - `S118` stuck-detector precision — observer-side detector refinement, orthogonal to forensic window reporting.

## Motivating Evidence

Implementation of the S116 survival ticket chain exposed a repeatable traceability gap:

1. After the stale exact-opportunity fix in `archive/tickets/S116DRIESCSUS-011.md`, the remaining baseline failure was a long-run authored-critical window, not a single-step planner contradiction.
2. Existing decision traces were strong enough to prove single-tick selection facts, but not strong enough to explain a 100–400 tick survival window without adding temporary ignored reproducers and local probe code.
3. The `Sleep` progress-barrier fix landed inside `archive/tickets/S116DRIESCSUS-012.md` required multiple rounds of ad hoc instrumentation just to distinguish stale-local opportunity bugs, ranking bias, threshold mismatch, and progress-barrier absence.

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

Add a derived, read-only report type in a new module `crates/worldwake-ai/src/survival_forensics.rs`, re-exported from `crates/worldwake-ai/src/lib.rs`. Placing it in `worldwake-ai/src/` — not under `tests/` — is deliberate: this mirrors how `DecisionTraceSink` and `ActionTraceSink` live as runtime types so both goldens (`worldwake-ai/tests/`) and the observer binary (`worldwake-cli/src/bin/observer.rs`) can consume the same model without duplicating it.

The window is keyed on `HomeostaticNeedId` (physiology-domain identifier used by `HomeostaticNeeds`, `DriveThresholds`, and `DeprivationExposure`), not on the AI-crate `RankedDriveKind`. The window itself is a physiology fact; ranking-domain data appears through `RankedGoalSnapshot` inside each frame.

```rust
pub struct CriticalWindowReport {
    pub agent: EntityId,
    pub need: HomeostaticNeedId,
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
    pub active_action: Option<ActiveActionSummary>,
    pub exhaustion_state: Option<ExhaustionSummary>,
    pub blocker_summary: Option<BlockerSummary>,
    pub local_authoritative_summary: LocalSurvivalStateSummary,
}

pub struct RankedGoalSnapshot {
    pub goal: GoalKey,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
    pub provenance_family: Option<RankedGoalProvenanceFamily>,
}

pub struct ActiveActionSummary {
    pub action_name: &'static str,
    pub instance: ActionInstanceId,
    pub started_at: Tick,
}

pub enum ExhaustionSummary {
    FrontierExhausted { expansions_used: u16 },
    BudgetExhausted { expansions_used: u16 },
    Unsupported,
}

pub struct BlockerSummary {
    pub blocker_count: u16,
    pub top_blocker: Option<BlockerKey>,
}
```

Typed summary enums are preferred over `String` for exhaustion and blocker fields so that downstream consumers (goldens, observer, future CI gates) can pattern-match against them without parsing. `RankedGoalSnapshot` carries the compact typed subset needed for survival competitor comparison; the detailed `RankedGoalProvenance` stays in the decision-trace sink for deep drill-down.

### D2: Shared extractor for authored-critical windows

Add a helper in `crates/worldwake-ai/src/survival_forensics.rs` that:

1. reads the agent's authored `DriveThresholds` (via the belief-view / ECS accessor)
2. finds maximal runs where a need stays `>= authored critical`
3. returns the top-N longest windows or a specific requested window

The helper runs independently of S117's anomaly detectors. S120's window definition is "need ≥ authored critical for N consecutive ticks"; S117's `AcuteNeedSpike` (30–99 tick sub-threshold) and `MaintenanceStarvation` (relief-rate deficit over 200-tick rolling windows) are different detection surfaces and neither subsumes the other. S120 does not consume S117 output.

The helper extends (not duplicates) the existing `SurvivalNeedRunTracker` and `assert_authored_critical_runs` support at `crates/worldwake-ai/tests/golden_harness/mod.rs` (tracker at line 70, assertion at line 149). Today that tracker records only `{need}_current` and `{need}_max` tick counts; it does not retain `start_tick`/`end_tick` or per-window frames. The extension relationship is:

- The runtime extractor (in `worldwake-ai/src/survival_forensics.rs`) becomes the authoritative producer of `CriticalWindowReport`. It extends the tracker's state with `start_tick` per need and emits a completed `CriticalWindowReport` when a run ends (or at run-end for runs still above critical).
- `SurvivalNeedRunTracker` continues to be used by goldens to accumulate the report-building state tick by tick. Its existing `{need}_max` counters remain useful for the current max-run assertions, which stay valid.
- The test-facing `assert_authored_critical_runs` helpers remain where they are; D5's new report-based assertions compose with them rather than replacing them.

This helper becomes the canonical way to investigate long-run survival failures in goldens.

### D3: Bounded frame capture rules

To keep reports legible and deterministic, capture:

- first 5 ticks of the window
- last 5 ticks of the window
- up to 5 evenly spaced interior ticks
- any tick where selected goal changes
- any tick where `selected_plan_source`, active action, blocker state, or exhaustion state changes

This avoids flooding while still showing causal changes.

Capture regime: per-tick observation hook invoked from the golden harness (parallel to how `SurvivalNeedRunTracker::observe` is called every tick in the current survival goldens — see `crates/worldwake-ai/tests/golden_survival_baseline.rs:145`). The hook reads the already-populated `DecisionTraceSink` and `ActionTraceSink` for the current tick plus the current authoritative physiology components, producing a candidate frame; the extractor then applies the bounded-capture rules above to decide which candidate frames survive into the final report. Event-log replay is not required.

### D4: Local authoritative survival summary

For each captured frame, include only the local authoritative state relevant to self-care diagnosis:

- actor effective place
- local water source presence / quantity
- local wash basin presence
- local sleep affordance presence if applicable
- local food source / edible stock presence if applicable

This summary exists to separate "planner failed despite local affordance" from "planner had no lawful local affordance and needed remote pursuit."

### D5: Golden/test integration

Add reusable assertion helpers under `crates/worldwake-ai/tests/golden_harness/mod.rs` that consume the runtime report types from `worldwake-ai::survival_forensics`. A failing survival golden can print a compact `CriticalWindowReport` directly from shared support instead of introducing bespoke ignored reproducers.

At minimum:

1. **Fatigue / Sleep progress-barrier test**: a synthetic authored-critical fatigue window SHALL produce a `CriticalWindowReport` such that at least one frame has `exhaustion_state == Some(ExhaustionSummary::FrontierExhausted { .. })` AND at least one frame has `selected_goal` whose `GoalKey::kind` matches `GoalKind::Sleep` (or a `ranked_goal_provenance_family` indicating the Sleep family).
2. **Wash-vs-water competition test**: a synthetic dirtiness authored-critical window SHALL produce a `CriticalWindowReport` such that at least one frame has `selected_goal` plus `top_competitors` collectively exposing both a wash-family goal and a water-acquire-family goal, and `selected_goal` matches one of those families deterministically.
3. **Bounded-capture determinism**: two runs of the same synthetic trace input SHALL produce byte-identical `CriticalWindowReport` vectors (via `PartialEq` or serialized comparison).

### D6: Optional observer/report rendering

If the existing observer binary is the best consumer, add a new optional section:

`## Section 9 — Critical Window Forensics`

This numbering follows the existing sequential `## Section N — Title` convention in `crates/worldwake-cli/src/bin/observer.rs` (sections 1–8 already defined, line 1278 onward). The new section renders only when requested, and only for the top-N longest authored-critical windows in the run. Because the report model lives in `worldwake-ai/src/survival_forensics.rs` (see D1), the observer can consume it via its existing `worldwake-ai` dependency — no crate-boundary refactor required.

Observer integration is optional for the first landing if the shared golden-support surface is sufficient, but the report model is designed so observer reuse is straightforward.

### D7: Documentation

Update `docs/golden-e2e-testing.md` and/or `docs/debugging-traces.md` to point survival-debugging work at the shared critical-window report helper rather than ad hoc ignored reproducers.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Inputs are existing decision traces (`DecisionTraceSink`), action traces (`ActionTraceSink`), authored `DriveThresholds`, per-tick `HomeostaticNeeds` values, and authoritative local-place state (water source / wash basin / sleep affordance / food source presence at the agent's effective place). All inputs are sampled through a per-tick observation hook invoked from the golden harness, the same way `SurvivalNeedRunTracker::observe` is called today. No new simulation information path is introduced; no agent planner reads this report.
2. **Positive-feedback analysis**: None. The report is read-only and post hoc, and nothing in the simulation consumes it.
3. **Concrete dampeners**: Bounded frame capture (D3) is the report dampener; it limits output volume without hiding causal changes.
4. **Stored state vs. derived read-model**:
   - **Stored/authored**: none beyond existing traces, scenario thresholds, `HomeostaticNeeds`, and `DeprivationExposure` (all authored or maintained by existing systems)
   - **Derived**: `CriticalWindowReport`, `CriticalWindowFrame`, `RankedGoalSnapshot`, `ExhaustionSummary`, `BlockerSummary`, `ActiveActionSummary`, `LocalSurvivalStateSummary` — all computed on demand from the stored inputs above and discardable without changing world meaning (FND-27).

## SystemFn Integration

None. This spec adds no `SystemFn`.

## Component Registration

None.

## Cross-System Interactions

- `worldwake-ai::survival_forensics` extracts planner/decision-side facts and authoritative physiology snapshots into the derived report model.
- `worldwake-ai/tests/golden_harness/mod.rs` wraps the runtime extractor with test-facing assertion helpers that compose with the existing `SurvivalNeedRunTracker` and `assert_authored_critical_runs` surfaces.
- Optional `worldwake-cli` observer rendering (D6) reads the same derived report model through its existing `worldwake-ai` dependency.
- No simulation system depends on the report. S117 and S118 do not consume it; S120 does not consume their output.

## Validation and Falsification

### Focused tests

1. A synthetic fatigue-critical window with repeated `Sleep` planning reports the correct authored threshold and at least one frame with `exhaustion_state == Some(ExhaustionSummary::FrontierExhausted { .. })` and at least one frame whose `selected_goal` resolves to the `Sleep` goal family (D5.1).
2. A synthetic thirst or dirtiness competition window reports both wash-family and water-acquire-family pressure through `selected_goal` plus `top_competitors`, using `RankedGoalSnapshot` for the competitor surface (D5.2).
3. Bounded frame selection is deterministic: two runs of the same synthetic trace input produce byte-identical `CriticalWindowReport` vectors (D5.3).

### Golden / integration tests

4. `golden_survival_baseline`, `golden_survival_scattered`, and `golden_survival_contested` support can emit a stable `CriticalWindowReport` for the longest authored-critical run when requested, via the per-tick observation hook described in D3.
5. A no-failure healthy run (no need ever crosses `authored_critical`) produces zero critical-window reports unless the caller explicitly requests the top-N windows, in which case the returned vector is empty.

## Outcome

Completed on 2026-04-18 via the archived ticket chain:

- [S120SURCRIWIN-001](/home/joeloverbeck/projects/worldwake/archive/tickets/S120SURCRIWIN-001.md)
- [S120SURCRIWIN-002](/home/joeloverbeck/projects/worldwake/archive/tickets/S120SURCRIWIN-002.md)
- [S120SURCRIWIN-003](/home/joeloverbeck/projects/worldwake/archive/tickets/S120SURCRIWIN-003.md)
- [S120SURCRIWIN-004](/home/joeloverbeck/projects/worldwake/archive/tickets/S120SURCRIWIN-004.md)

### Landed changes

- **D1-D4 — Runtime forensic surface**: added `crates/worldwake-ai/src/survival_forensics.rs`, exported `CriticalWindowReport`, `CriticalWindowFrame`, `SurvivalForensicExtractor`, bounded frame capture, and the authoritative local survival summary through `worldwake-ai/src/lib.rs`.
- **D5 — Golden/test integration**: added the shared test-facing helper module `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`, wired per-tick observation into the three survival golden binaries, and landed focused proof for the sleep progress-barrier, wash-vs-water competition, and deterministic bounded-capture contracts.
- **D6 — Observer rendering**: added observer Section 9 (`## Section 9 — Critical Window Forensics`) plus `--critical-window-top-n` in `crates/worldwake-cli/src/bin/observer.rs`, using the same runtime report model rather than a duplicate consumer path.
- **D7 — Documentation**: updated `docs/golden-e2e-testing.md` and `docs/debugging-traces.md` so survival-debugging work points at the canonical runtime/report helper and observer surface instead of bespoke ignored reproducers.

### Deviations from original plan

- `ActiveActionSummary.action_name` landed as `String` rather than `&'static str`, matching the live action-trace carrier shape.
- The D5.2 competition contract was narrowed during implementation to the honest extractor boundary: wash-vs-water competition is exposed by `selected_goal` plus `top_competitors` collectively, because the selected goal is intentionally excluded from `top_competitors`.
- The observer integration described as optional in D6 was completed as part of the initial S120 delivery rather than deferred.

### Verification

- Runtime module verification, focused golden/observer verification, and full workspace verification were completed across the archived S120 ticket chain, including:
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-cli --bin observer`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- The long-running `golden_survival_*` 1440-tick scenarios remained `#[ignore]` during the workspace-wide verification runs, and that limitation is recorded explicitly in the archived ticket closeouts where it applied.
