# S120SURCRIWIN-002: Golden harness forensic assertions and focused survival tests

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — this ticket adds test-facing wrappers under `crates/worldwake-ai/tests/golden_harness/` composing over the runtime types from `S120SURCRIWIN-001`, plus three focused tests. No simulation, planner, or agent-decision behavior changes.
**Deps**: `S120SURCRIWIN-001`

## Problem

After `S120SURCRIWIN-001` lands the runtime report model, the three survival goldens (`golden_survival_baseline`, `golden_survival_scattered`, `golden_survival_contested`) still have no shared path to emit a `CriticalWindowReport` when an authored-critical run assertion fails. Without that path, future long-run survival failures still require bespoke `#[ignore]`'d reproducers and local probe code — which is exactly the traceability gap S120 exists to close.

This ticket implements S120 deliverable D5 in full: test-facing assertion helpers that wrap the runtime extractor, per-tick `observe` hook wiring into the three survival goldens, and three focused tests proving the report surface works for the specific survival contradiction classes S116 surfaced.

## Assumption Reassessment (2026-04-18)

1. Existing survival goldens and their tracker plumbing:
   - `crates/worldwake-ai/tests/golden_survival_baseline.rs` — calls `critical_need_runs.observe(...)` at line 145; `assert_authored_critical_runs` at line 295.
   - `crates/worldwake-ai/tests/golden_survival_scattered.rs` — same pattern.
   - `crates/worldwake-ai/tests/golden_survival_contested.rs` — same pattern.
   - Shared harness: `crates/worldwake-ai/tests/golden_harness/mod.rs` holds `SurvivalNeedRunTracker` (line 70), `update_need_run` (line 130), and `assert_authored_critical_runs` (line 149) + `assert_authored_critical_runs_with_overrides` (line 164). These are called per-tick by each golden.
   Validated during `/reassess-spec` pass on 2026-04-18.
2. Spec references: `specs/S120-survival-critical-window-forensics.md` D5 (lines 170–180) defines three focused-test invariants that this ticket implements directly:
   - D5.1: Fatigue / Sleep progress-barrier test — at least one frame with `exhaustion_state == Some(ExhaustionSummary::FrontierExhausted { .. })` AND at least one frame with `selected_goal` matching the `Sleep` goal family.
   - D5.2: Wash-vs-water competition test — `top_competitors` contains both a wash-family and water-acquire-family goal; `selected_goal` matches one of them (deterministic).
   - D5.3: Bounded-capture determinism — two runs of the same synthetic trace input produce byte-identical `CriticalWindowReport` vectors.
3. Shared abstraction boundary under audit: the golden harness's per-tick observation loop. The new `SurvivalForensicExtractor::observe` hook slots in alongside the existing `SurvivalNeedRunTracker::observe` call; both operate on the same per-tick inputs (needs, thresholds, trace sinks) and neither replaces the other.
6. AI-regression coverage layer: the three focused tests are focused unit tests against synthetic `SurvivalForensicExtractor` inputs (not full `agent_tick` integration). The golden-harness wiring is separately covered by the existing `all_agents_survive_1440_ticks` regressions — which continue to pass unchanged, since the extractor is optional and does not affect `SurvivalNeedRunTracker`'s existing assertion semantics.
12. Scenario isolation (precision-rules Rule 8): D5.1 uses a synthetic fatigue-critical trace — it does not require modifying `scenarios/survival-baseline.ron`. D5.2 uses a synthetic wash-vs-water competition trace — it does not require modifying `scenarios/survival-contested.ron`. This keeps the focused tests independent of scenario calibration changes in other spec chains.
13. Adjacent contradiction audit: the existing `assert_authored_critical_runs` max-run assertions remain load-bearing — this ticket does not weaken or alter them. The forensic report is emitted on-demand (either explicitly requested, or on assertion failure via a future panic-hook opt-in); it is not a replacement for the max-run bound.

## Architecture Check

1. Keeping assertion helpers composable (rather than replacing `assert_authored_critical_runs`) means existing goldens continue to enforce the max-run bound and the forensic surface is an additive diagnostic — reviewers can land this ticket without reasoning about changes to current passing goldens.
2. Synthetic focused tests over `SurvivalForensicExtractor::observe` rather than full-scenario integration tests mean the three D5 assertions are fast, deterministic, and not gated on scenario calibration drift. Scenario-driven validation is already covered by the existing `all_agents_survive_1440_ticks` goldens.
3. Per-tick hook integration into the three goldens reuses the existing `SurvivalNeedRunTracker::observe` call-site pattern — no new iteration of the tick loop, no new snapshot-capture regime. The observation hook is the same regime the spec's Section H item 1 specified.

## Verification Layers

1. D5.1 — Sleep/FrontierExhausted invariant → focused unit test over `SurvivalForensicExtractor` with a synthetic fatigue-critical trace including a `PlanSearchOutcome::FrontierExhausted` event.
2. D5.2 — wash-vs-water competition → focused unit test over `SurvivalForensicExtractor` with a synthetic dirtiness-critical trace including two competitor `RankedGoal` entries.
3. D5.3 — deterministic frame capture → focused unit test running `observe` twice with the same synthetic inputs and asserting `PartialEq` on the resulting `Vec<CriticalWindowReport>`.
4. Golden harness regression → existing `all_agents_survive_1440_ticks` tests in the three survival goldens remain green; the new `observe` hook does not perturb `SurvivalNeedRunTracker`'s existing `{need}_max` counters.
5. No decision/action-trace layer mapping is applicable here — these are synthetic focused tests, not runtime planner coverage.

## What to Change

### 1. Test-facing assertion wrappers

Add a new file `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` (or append to `mod.rs` if small), re-exporting from `mod.rs`. Public helpers:

- `fn observe_critical_windows(extractor: &mut SurvivalForensicExtractor, ...)` — thin wrapper matching the existing `update_need_run` helper style.
- `fn expect_sleep_progress_barrier_window(reports: &[CriticalWindowReport])` — asserts D5.1 invariant.
- `fn expect_wash_vs_water_competition_window(reports: &[CriticalWindowReport])` — asserts D5.2 invariant.
- `fn expect_deterministic_reports(a: &[CriticalWindowReport], b: &[CriticalWindowReport])` — asserts D5.3 byte-equality.
- `fn dump_reports_for_debug(reports: &[CriticalWindowReport]) -> String` — compact text summary for `#[ignore]`-free debugging; printed on assertion failure.

### 2. Per-tick hook wiring in the three survival goldens

In each of `golden_survival_baseline.rs`, `golden_survival_scattered.rs`, `golden_survival_contested.rs`, alongside the existing `SurvivalNeedRunTracker::observe` call:

- Construct `SurvivalForensicExtractor::new(agent)` once per agent at the start of the run.
- Call `extractor.observe(tick, needs, thresholds, decision_trace_for_agent_at_tick, action_trace_snapshot, local_state)` inside the same per-tick loop.
- Call `extractor.finalize()` at run end and attach to the observation.
- Extend `AgentSurvivalObservation` with an additional `critical_window_reports: Vec<CriticalWindowReport>` field.

No existing assertion is modified. The reports are captured and available for inspection but only asserted on in future tickets that opt into the new surface.

### 3. Focused test: D5.1 — Sleep progress-barrier fatigue window

Add `crates/worldwake-ai/tests/forensic_sleep_progress_barrier.rs`. Construct a synthetic `SurvivalForensicExtractor` session with:
- a 200-tick synthetic `HomeostaticNeeds` sequence where `fatigue` crosses `DriveThresholds::default().fatigue.critical()`
- a synthetic `AgentDecisionTrace` per tick with `PlanSearchOutcome::FrontierExhausted { expansions_used }` and `selected_goal` matching `GoalKey::from(GoalKind::Sleep { .. })`

Call `finalize()` and invoke `expect_sleep_progress_barrier_window(&reports)`. Assert the test passes.

### 4. Focused test: D5.2 — wash-vs-water competition window

Add `crates/worldwake-ai/tests/forensic_wash_vs_water_competition.rs`. Construct a synthetic 150-tick session with:
- `dirtiness` at authored critical throughout
- each tick's trace surfaces two `RankedGoal` entries: one wash-family, one water-acquire-family
- `selected_goal` alternates deterministically between them (to prove competitor reporting even under selection churn)

Call `finalize()` and invoke `expect_wash_vs_water_competition_window(&reports)`. Assert the test passes.

### 5. Focused test: D5.3 — bounded-capture determinism

Add `crates/worldwake-ai/tests/forensic_determinism.rs`. Run the same synthetic 100-tick observe sequence twice over two independent `SurvivalForensicExtractor` instances, call `finalize()` on both, and invoke `expect_deterministic_reports(&a, &b)`. Assert the test passes.

### 6. Register golden-harness helpers

Update `crates/worldwake-ai/tests/golden_harness/mod.rs` with `pub mod survival_forensics_assertions;` (if split into its own file) or inline the helpers at the bottom of `mod.rs`. Re-export assertion functions from the module root so goldens can `use golden_harness::expect_sleep_progress_barrier_window;`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add helper re-exports)
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` (new)
- `crates/worldwake-ai/tests/golden_survival_baseline.rs` (modify — add per-tick hook + observation field)
- `crates/worldwake-ai/tests/golden_survival_scattered.rs` (modify — same)
- `crates/worldwake-ai/tests/golden_survival_contested.rs` (modify — same)
- `crates/worldwake-ai/tests/forensic_sleep_progress_barrier.rs` (new)
- `crates/worldwake-ai/tests/forensic_wash_vs_water_competition.rs` (new)
- `crates/worldwake-ai/tests/forensic_determinism.rs` (new)

## Out of Scope

- Modification of `SurvivalNeedRunTracker` or `assert_authored_critical_runs` at `crates/worldwake-ai/tests/golden_harness/mod.rs:70,149`. Those assertions remain load-bearing; this ticket composes alongside them.
- Any scenario (`scenarios/*.ron`) modification.
- Observer binary Section 9 rendering — see `S120SURCRIWIN-003`.
- Documentation updates — see `S120SURCRIWIN-004`.
- Turning survival-goldens into `CriticalWindowReport`-asserting tests. This ticket only adds the capture + focused-test surface; wholesale conversion of existing `max_run` assertions to report-based assertions is a future concern.
- Runtime changes to the report model or extractor — all lives in the `worldwake-ai::survival_forensics` module landed by `S120SURCRIWIN-001`.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test forensic_sleep_progress_barrier` — D5.1 focused test green.
2. `cargo test -p worldwake-ai --test forensic_wash_vs_water_competition` — D5.2 focused test green.
3. `cargo test -p worldwake-ai --test forensic_determinism` — D5.3 focused test green.
4. `cargo test -p worldwake-ai --test golden_survival_baseline` — existing baseline golden remains green.
5. `cargo test -p worldwake-ai --test golden_survival_scattered` — existing scattered golden remains green.
6. `cargo test -p worldwake-ai --test golden_survival_contested` — existing contested golden remains green.
7. Existing suite: `cargo test --workspace` remains green.
8. Lint: `cargo clippy --workspace --all-targets -- -D warnings` passes.

### Invariants

1. The three survival goldens continue to enforce `assert_authored_critical_runs` (or `_with_overrides`) on `SurvivalNeedRunTracker` data — existing max-run bounds are unchanged.
2. The `SurvivalForensicExtractor` is invoked per tick but its output is captured without asserting; failure modes remain unchanged until explicit report-based assertions are added in a future ticket.
3. D5.3 determinism: two independent synthetic observation sequences with identical inputs produce `PartialEq`-equal `Vec<CriticalWindowReport>` outputs.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/forensic_sleep_progress_barrier.rs` — D5.1 invariant (Sleep / FrontierExhausted).
2. `crates/worldwake-ai/tests/forensic_wash_vs_water_competition.rs` — D5.2 invariant (competing goals in frame).
3. `crates/worldwake-ai/tests/forensic_determinism.rs` — D5.3 invariant (byte-identical outputs).
4. `crates/worldwake-ai/tests/golden_survival_baseline.rs` — per-tick hook wiring + new observation field (no new assertion).
5. `crates/worldwake-ai/tests/golden_survival_scattered.rs` — same pattern.
6. `crates/worldwake-ai/tests/golden_survival_contested.rs` — same pattern.

### Commands

1. `cargo test -p worldwake-ai --test forensic_sleep_progress_barrier --test forensic_wash_vs_water_competition --test forensic_determinism` — targeted D5 focused tests.
2. `cargo test -p worldwake-ai --test golden_survival_baseline --test golden_survival_scattered --test golden_survival_contested` — regression sweep over the three goldens that now wire the hook.
3. `cargo test -p worldwake-ai` — full AI crate.
4. `cargo clippy --workspace --all-targets -- -D warnings` — workspace lint (CI parity).
5. `cargo test --workspace` — full regression sweep.
