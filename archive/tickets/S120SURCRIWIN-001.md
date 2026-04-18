# S120SURCRIWIN-001: Runtime survival-forensics module

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — the new module `worldwake-ai::survival_forensics` is a read-only derived read-model over existing decision traces, action traces, authoritative physiology components, and authoritative local-place state. It adds no simulation behavior, no planner behavior, and no new authoritative state.
**Deps**: None

## Problem

There is no stable, reusable read-model for "why did this agent's need stay above authored critical for this long?" Today a prolonged authored-critical window can only be explained by hand — decision traces prove single-tick facts, and `SurvivalNeedRunTracker` in the golden harness records `{need}_current`/`{need}_max` tick counts but not the `start_tick`/`end_tick` of a window, the selected goal per frame, competing goals, active action state, plan-search exhaustion state, blocker summary, or local-place affordance presence. Each long-run survival failure in S116 required one-off `#[ignore]`'d reproducers and local probe code instead of a mechanical forensic surface.

This ticket introduces the runtime forensic surface defined in S120 deliverables D1–D4: report model types, extractor, bounded frame capture, and local survival summary. Once this surface lands, S120SURCRIWIN-002 can wire it into the survival goldens and S120SURCRIWIN-003 can consume it from the observer binary.

## Assumption Reassessment (2026-04-18)

1. All referenced types exist at the paths cited in the spec:
   - `HomeostaticNeedId` / `HomeostaticNeeds` — `crates/worldwake-core/src/needs.rs:9,19`
   - `DriveThresholds` — `crates/worldwake-core/src/drives.rs:58`
   - `DeprivationExposure` — `crates/worldwake-core/src/needs.rs:92`
   - `GoalKey` — `crates/worldwake-core/src/goal.rs:142`
   - `RankedGoal` — `crates/worldwake-ai/src/goal_model.rs:2520`
   - `GoalPriorityClass` — `crates/worldwake-ai/src/goal_model.rs:2249`
   - `RankedGoalProvenanceFamily` — `crates/worldwake-ai/src/goal_model.rs:33`
   - `SelectedPlanSource` — `crates/worldwake-ai/src/decision_trace.rs:1068`
   - `PlanSearchOutcome` — `crates/worldwake-ai/src/decision_trace.rs:955`
   - `BlockerKey` — `crates/worldwake-core/src/blocked_intent.rs:11`
   - `ActionInstanceId` — `crates/worldwake-sim/src/action_ids.rs:6`
   - `DecisionTraceSink` / `AgentDecisionTrace` — `crates/worldwake-ai/src/decision_trace.rs:1218,72`
   - `ActionTraceSink` — `crates/worldwake-sim/src/action_trace.rs` (re-exported from `worldwake-sim/src/lib.rs:86`)
   All validated during the `/reassess-spec` pass on 2026-04-18.
2. Spec reference: `specs/S120-survival-critical-window-forensics.md` Deliverables D1 (lines 71–124), D2 (127–145), D3 (148–157), D4 (160–167). The "Dual-Use Read-Model Types" pattern in `.claude/skills/reassess-spec/references/worldwake-validation-patterns.md` mandates runtime placement (`src/`) rather than `tests/`, which this ticket honors. Analog patterns already resident in the codebase: `DecisionTraceSink` (`worldwake-ai/src/decision_trace.rs`) and `ActionTraceSink` (`worldwake-sim/src/action_trace.rs`).
3. Shared abstraction boundary under audit: the derived read-model over `(DecisionTraceSink × ActionTraceSink × per-tick HomeostaticNeeds × authoritative local-place affordances × authored DriveThresholds)`. This ticket does not widen that boundary — it only names it as a type and exposes an extractor that reads it.
8. No heuristic or filter is removed or bypassed. `SurvivalNeedRunTracker` continues to exist at `crates/worldwake-ai/tests/golden_harness/mod.rs:70`; its `{need}_current`/`{need}_max` counters remain load-bearing for the existing `assert_authored_critical_runs` helper at line 149. This ticket introduces new start-tick tracking via the runtime extractor; it does not mutate `SurvivalNeedRunTracker`'s existing fields.
13. Adjacent contradiction audit: S117's `AcuteNeedSpike` (30–99 tick sub-threshold) and `MaintenanceStarvation` (rolling 200-tick relief-rate deficit) detectors are observer-side anomaly surfaces with different window definitions than S120's "need ≥ authored critical for N ticks". S120 runs independently of S117; neither consumes the other. This is the in-scope consequence captured in the spec's Dependencies section.
15. Cumulative arithmetic: each agent tick the physiology system updates `HomeostaticNeeds` by per-profile rates; `DeprivationExposure.{need}_critical_ticks` is incremented by `needs_system` whenever a need meets its critical threshold. The forensic window's `start_tick` is the tick at which `HomeostaticNeeds.value(need) >= DriveThresholds.{need}.critical()` first holds after a non-critical tick; the `end_tick` is the last tick the condition holds before dropping. The report itself performs no arithmetic on authoritative state — it only samples and summarizes.

## Architecture Check

1. Runtime placement in `worldwake-ai/src/survival_forensics.rs` (rather than under `tests/`) is architecturally required: the observer binary in `worldwake-cli/src/bin/observer.rs` cannot import test modules from sibling crates. Runtime placement also mirrors the established pattern for `DecisionTraceSink`/`ActionTraceSink`, which are likewise read-model types consumed by both goldens and the observer. This placement is enforced by the "Dual-Use Read-Model Types" pattern recorded in the reassess-spec skill.
2. The report is strictly derived (FND-27): deletion of the extractor and all `CriticalWindowReport` values leaves world meaning unchanged because no simulation system reads them. Typed summary enums (`ExhaustionSummary`, `BlockerSummary`, `ActiveActionSummary`) avoid the weaker `Option<String>` alternative and keep the report machine-parseable.
3. No backwards-compatibility shim: `SurvivalNeedRunTracker` and `assert_authored_critical_runs` remain untouched; the new extractor composes alongside them rather than replacing them. The `max` assertions continue to hold regardless of whether the extractor is invoked.
4. Determinism (CLAUDE.md "Critical Invariants"): the extractor stores frames in a `Vec<CriticalWindowFrame>` in tick order, uses `BTreeMap<HomeostaticNeedId, _>` for per-need window state, and does no floating-point arithmetic. Two runs of the same synthetic trace input produce byte-identical report vectors — this is enforced as a D5.3 test in ticket `S120SURCRIWIN-002`.

## Verification Layers

1. Report model compiles and serializes deterministically → focused unit tests in `crates/worldwake-ai/src/survival_forensics.rs` (`#[cfg(test)]` block) asserting `PartialEq` equivalence for identical trace inputs.
2. Extractor correctly identifies window `start_tick`/`end_tick` from a synthetic tick sequence → focused unit test with a hand-crafted `HomeostaticNeeds` sequence crossing the critical threshold.
3. Bounded frame capture (first-5 / last-5 / up-to-5 interior / all change points) selects the correct frames from a synthetic 100-tick window → focused unit test asserting the exact captured tick list.
4. `LocalSurvivalStateSummary` samples the agent's effective place and records affordance presence → focused unit test constructing a minimal topology fixture.
5. No behavioral claims about runtime planner/ranking/agent-decision code — this module does not touch those paths, so cross-layer trace mapping is not applicable. Stated per precision-rules Rule 5.

## What to Change

### 1. New module `crates/worldwake-ai/src/survival_forensics.rs`

Add a new file declaring the derived report model and extractor. Public types:

- `CriticalWindowReport { agent: EntityId, need: HomeostaticNeedId, start_tick: Tick, end_tick: Tick, threshold: Permille, peak_value: Permille, frames: Vec<CriticalWindowFrame> }`
- `CriticalWindowFrame { tick: Tick, need_value: Permille, selected_goal: Option<GoalKey>, selected_plan_source: Option<SelectedPlanSource>, top_competitors: Vec<RankedGoalSnapshot>, active_action: Option<ActiveActionSummary>, exhaustion_state: Option<ExhaustionSummary>, blocker_summary: Option<BlockerSummary>, local_authoritative_summary: LocalSurvivalStateSummary }`
- `RankedGoalSnapshot { goal: GoalKey, priority_class: GoalPriorityClass, motive_score: u32, provenance_family: Option<RankedGoalProvenanceFamily> }`
- `ActiveActionSummary { action_name: String, instance: ActionInstanceId, started_at: Tick }`
- `ExhaustionSummary { FrontierExhausted { expansions_used: u16 }, BudgetExhausted { expansions_used: u16 }, Unsupported }`
- `BlockerSummary { blocker_count: u16, top_blocker: Option<BlockerKey> }`
- `LocalSurvivalStateSummary { place: EntityId, water_source_present: bool, wash_basin_present: bool, sleep_affordance_present: bool, food_source_present: bool }`

All types derive `Clone, Debug, Eq, PartialEq` and `Serialize, Deserialize` where field types support them (matching the existing `decision_trace.rs` convention). No `Hash`, no `Copy` on non-trivial types, no `f32`/`f64`.

### 2. Extractor state and construction

Add `pub struct SurvivalForensicExtractor` with per-need `BTreeMap<HomeostaticNeedId, WindowBuilder>` state. `WindowBuilder` retains the active window's `start_tick`, `peak_value`, candidate frames, and prior-tick sampled fields (for change-detection). Public API:

- `SurvivalForensicExtractor::new(agent: EntityId) -> Self`
- `fn observe(&mut self, tick: Tick, needs: &HomeostaticNeeds, thresholds: &DriveThresholds, decision_trace: Option<&AgentDecisionTrace>, action_trace_snapshot: &ActionTraceSnapshot<'_>, local_state: &LocalSurvivalStateSummary)` — invoked per tick from the golden harness (D3). Appends a candidate frame and closes any window whose need dropped below `critical`.
- `fn finalize(self) -> Vec<CriticalWindowReport>` — closes all still-open windows and returns top-N longest windows sorted by `end_tick - start_tick`.
- `fn top_n_longest(reports: &[CriticalWindowReport], n: usize) -> Vec<&CriticalWindowReport>` — deterministic selector.

`ActionTraceSnapshot` is a borrowed view over the `ActionTraceSink`'s events for the current tick; prefer a thin wrapper struct in the same module to avoid re-exposing internal sink shape.

### 3. Bounded frame capture (D3)

Implement frame-filtering in `WindowBuilder::flush` per the spec rules:
- always retain first 5 captured ticks
- always retain last 5 captured ticks
- retain up to 5 evenly spaced interior ticks (deterministic: choose by fixed stride `max(1, (window_len - 10) / 5)`)
- retain any tick where `selected_goal`, `selected_plan_source`, `active_action`, `exhaustion_state`, or `blocker_summary` differs from the prior captured frame

Ordering is strictly ascending by tick with no duplicates.

### 4. `LocalSurvivalStateSummary` capture (D4)

The summary samples the agent's effective place at observation time. Use existing belief-view / world accessors to query:
- water source presence / quantity → `resource_sources` at the effective place
- wash basin presence → `WorkstationTag::WashBasin` presence
- sleep affordance presence → place tag / workstation tag per the survival scenario convention
- food source / edible stock presence → resource source + local item lots

The summary is a pure read of authoritative world state at the current tick (FND-14A: same-tick co-located observation is belief-equivalent for physical properties). It does not inform agent decisions.

### 5. Module export

Add `pub mod survival_forensics;` to `crates/worldwake-ai/src/lib.rs` and a matching `pub use survival_forensics::{ActionTraceSnapshot, CriticalWindowReport, CriticalWindowFrame, RankedGoalSnapshot, ActiveActionSummary, ExhaustionSummary, BlockerSummary, LocalSurvivalStateSummary, SurvivalForensicExtractor};` re-export block.

### 6. Focused unit tests (in-module `#[cfg(test)]`)

Add unit tests covering:
- `start_tick`/`end_tick` detection on a 20-tick synthetic sequence
- bounded frame capture on a 100-tick synthetic window — exact tick list asserted
- change-point capture on a 20-tick window with a goal switch at tick 10
- `ExhaustionSummary` / `BlockerSummary` / `ActiveActionSummary` population from synthetic trace inputs
- `top_n_longest` deterministic ordering with tied window lengths

## Files to Touch

- `crates/worldwake-ai/src/survival_forensics.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add `pub mod` + `pub use` lines)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — derive `Serialize` / `Deserialize` on `SelectedPlanSource` so the new report model can retain serde support on `selected_plan_source`)

## Out of Scope

- Golden harness integration and focused survival tests — see `S120SURCRIWIN-002`.
- Observer binary Section 9 rendering — see `S120SURCRIWIN-003`.
- Documentation updates (`docs/golden-e2e-testing.md`, `docs/debugging-traces.md`) — see `S120SURCRIWIN-004`.
- Any modification to `SurvivalNeedRunTracker` or `assert_authored_critical_runs` at `crates/worldwake-ai/tests/golden_harness/mod.rs` — the extractor composes alongside them, it does not replace them.
- Changes to `needs_system`, ranking, planner, affordance generation, or any agent-decision code.
- Persisting forensic reports in authoritative state or the event log.
- Cross-consumption with S117 anomaly detectors (independent detection surfaces — see spec Dependencies).

## Acceptance Criteria

### Tests That Must Pass

1. All new focused tests inside `crates/worldwake-ai/src/survival_forensics.rs` pass.
2. `cargo test -p worldwake-ai --lib survival_forensics::tests` completes with all new unit tests green.
3. Existing suite: `cargo test --workspace` remains green (no regression — this is a pure additive change).
4. Lint: `cargo clippy --workspace --all-targets -- -D warnings` passes.

### Invariants

1. `CriticalWindowReport` and all nested types are strictly derived (FND-27): no authoritative simulation system reads them; deletion changes no world meaning.
2. Determinism: two `observe` sequences with identical inputs produce `PartialEq`-equal `Vec<CriticalWindowReport>` outputs. No `HashMap`/`HashSet` in extractor state; no floats; no wall-clock reads.
3. Runtime placement (not test-only): `survival_forensics.rs` lives under `crates/worldwake-ai/src/`, is `pub mod`-declared in `lib.rs`, and can be imported by `worldwake-cli`'s observer binary.
4. Report model does not encode magic numbers — thresholds and peak values come from the agent's authored `DriveThresholds` and authoritative `HomeostaticNeeds`, never hardcoded.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/survival_forensics.rs` (`#[cfg(test)]` block) — focused unit tests for window detection, bounded frame capture, change-point detection, summary population, and deterministic top-N selection.

### Commands

1. `cargo test -p worldwake-ai --lib survival_forensics` — targeted unit tests.
2. `cargo test -p worldwake-ai` — full AI crate test suite.
3. `cargo clippy --workspace --all-targets -- -D warnings` — workspace lint (CI parity).
4. `cargo test --workspace` — full workspace regression sweep.

## Outcome

Completed on 2026-04-18.

- Added `crates/worldwake-ai/src/survival_forensics.rs` with the runtime critical-window report model, bounded-frame extractor, deterministic top-N selector, and authoritative local survival summary capture.
- Exported the new runtime surface from `crates/worldwake-ai/src/lib.rs`, including the thin `ActionTraceSnapshot` wrapper used to hand current active-action state and same-tick action events into the extractor without exposing `ActionTraceSink` internals directly.
- Added six focused in-module tests covering window start/end detection, bounded frame capture, change-point retention, summary population, deterministic top-N tie-breaking, and authoritative local survival summary capture.
- Landed same-crate shared-shape fallout in `crates/worldwake-ai/src/decision_trace.rs` by deriving `Serialize` / `Deserialize` on `SelectedPlanSource`, which keeps the report model aligned with the ticketed `selected_plan_source` field instead of introducing a wrapper enum or dropping serde on the new report types.

## Deviations

- `ActiveActionSummary.action_name` ships as `String` rather than `&'static str` because the live action and decision traces carry action names as owned strings; forcing a `'static` string here would have required an artificial conversion.
- `LocalSurvivalStateSummary.sleep_affordance_present` follows the current prototype-world place-tag convention (`Inn`, `Barracks`, `Camp`) instead of inventing a new authoritative sleep-affordance carrier. The underlying runtime `sleep` action remains available per the existing action registry contract.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib survival_forensics::tests`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
