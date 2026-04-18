# S120SURCRIWIN-003: Observer Section 9 — Critical Window Forensics rendering

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None — this ticket adds an optional rendering section to the observer binary; no simulation, planner, or agent-decision behavior changes. The observer itself is a passive read-only tool (FND-26).
**Deps**: `archive/tickets/S120SURCRIWIN-001.md`, `archive/tickets/S120SURCRIWIN-002.md`

## Problem

The spec marks observer integration as optional for the first landing of S120 (`specs/S120-survival-critical-window-forensics.md` D6: "Observer integration is optional for the first landing if the shared golden-support surface is sufficient"). This ticket closes D6 so that a scenario-analysis session — driven by `/scenario-analysis` or the manual observer dump pipeline — can drill into top-N longest authored-critical windows without re-implementing the forensic helper in a different consumer.

The observer binary today ends at `## Section 8 — Budget Exhaustion Snapshots` (`crates/worldwake-cli/src/bin/observer.rs:349,2111`). Adding Section 9 is the natural next-integer extension and follows the established `## Section N — Title` convention enforced by `/scenario-analysis`'s anomaly-parsing conventions.

## Assumption Reassessment (2026-04-18)

1. Observer binary structure:
   - `crates/worldwake-cli/src/bin/observer.rs` — existing sections 1 through 8 render via `writeln!(out, "## Section N — ...")` markers at lines 1278, 1304, 1451, 1473, 1640, 1744, 1817, and 349 (Section 8). The file ends around line 2111+ after Section 8's budget-exhaustion rendering. Validated during `/reassess-spec` pass on 2026-04-18.
   - `worldwake-cli`'s `Cargo.toml` already depends on `worldwake-ai`, so `worldwake-ai::survival_forensics` is importable after `S120SURCRIWIN-001` lands.
2. Spec reference: `specs/S120-survival-critical-window-forensics.md` D6 (lines 183–191) specifies `## Section 9 — Critical Window Forensics`. The reassessment confirmed section numbering follows sequential integers (1, 2, 3, ..., 8) — the original spec draft's "Section 2.5" suggestion was corrected to "Section 9" during reassessment per output-format-fidelity rules.
3. Shared abstraction boundary: the observer consumes the same `CriticalWindowReport` type that golden tests consume. Reassessment against the live `observer.rs` loop showed the observer already has the same per-tick world, scheduler, decision-trace, and action-trace context that the golden harness uses, so the honest extraction path is live per-tick observation into `SurvivalForensicExtractor`, not a second replay-only path. The canonical report model remains identical across both consumers.
13. Adjacent contradiction audit: S117's observer Section 3 anomaly detectors are independent of this section. S120 Section 9 renders forensic drill-down for top-N longest authored-critical windows; S117 Section 3 renders anomaly detections. Both may render in the same observer dump without coupling. This is the in-scope consequence captured in the spec's Dependencies section.

## Architecture Check

1. Keeping the section optional (renders only when requested, and only for top-N longest windows per the spec) preserves observer readability for scenarios without survival distress. Every other existing section is unconditional; Section 9 being opt-in is a deliberate scale choice.
2. Consuming the runtime `survival_forensics` module rather than re-implementing extraction in the observer binary preserves one canonical extraction path — deleting Section 9 does not weaken the goldens' surface and vice versa.
3. Deferring this work until after `S120SURCRIWIN-001` and `S120SURCRIWIN-002` prove the surface in goldens is intentional: the observer section is downstream of the canonical surface, not its own source of truth.

## Verification Layers

1. Section 9 renders correctly for a report set with at least one authored-critical window → focused `observer.rs` test constructs a synthetic `CriticalWindowReport` and asserts the Section 9 output matches an expected snippet.
2. Section 9 renders an empty-state line for a healthy run with no authored-critical windows → focused `observer.rs` test passes an empty report slice.
3. Section numbering continuity → focused `observer.rs` formatting test asserts Section 8 appears before Section 9 in rendered output. No cross-layer mapping applies — observer is a pure read-only rendering surface.

## What to Change

### 1. Extract observer-side window reports

Add a helper at the observer side (local to the `observer.rs` binary unless the code grows materially) that feeds the existing per-tick observer loop into the same `SurvivalForensicExtractor::observe` calls the goldens use. Reuse the runtime extractor; do not re-implement detection or add a second replay-only extraction path.

### 2. Section 9 rendering

After the Section 8 rendering block (around line 2111+), add:

```
## Section 9 — Critical Window Forensics

<per-agent top-N windows rendered as markdown subsections>
```

Each rendered window includes: agent name, need, `start_tick`..`end_tick`, authored threshold, peak value, selected-goal summary across captured frames, typed exhaustion state, typed blocker summary, and local authoritative summary at key frames.

Top-N is configurable via an observer command-line flag (e.g., `--critical-window-top-n <N>`, default `3`). If no authored-critical windows exist, render a single empty-state line: `No authored-critical windows detected.`

### 3. Observer-level focused tests

Add focused tests in `crates/worldwake-cli/src/bin/observer.rs` covering:
- Section 9 renders for a synthetic dump containing one window (assert expected header + content)
- Section 9 renders empty-state for a healthy dump
- Section numbering sequence: Section 8 header appears before Section 9 header in the output

### 4. Scenario-analysis integration note (not code)

Not in this ticket — documentation of the new section's consumption contract by `/scenario-analysis` is deferred to `S120SURCRIWIN-004`.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — add Section 9 extraction/rendering block and focused tests)
- `crates/worldwake-cli/src/observer/survival_forensics_view.rs` (new, optional — only if the helper surface stops being small enough to keep local)

## Out of Scope

- Changes to `worldwake-ai::survival_forensics` module internals — consume it as-is from `S120SURCRIWIN-001`.
- Changes to goldens or their harness — see `S120SURCRIWIN-002`.
- Documentation updates — see `S120SURCRIWIN-004`.
- Integration with S117 observer anomaly detectors. Section 9 is independent of Section 3; cross-reference between the two is future work.
- Making Section 9 render unconditionally. Per spec, it renders only when windows exist or when explicitly requested.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli --bin observer` — focused observer formatting tests pass, including the new Section 9 coverage.
2. Existing observer regression tests (if any): `cargo test -p worldwake-cli` remains green.
3. Existing suite: `cargo test --workspace` remains green.
4. Lint: `cargo clippy --workspace --all-targets -- -D warnings` passes.

### Invariants

1. Section numbering continuity: `## Section 9 — Critical Window Forensics` appears after `## Section 8` in observer source order and in rendered output.
2. Section 9 renders from the `worldwake-ai::survival_forensics` module (canonical extractor); no duplicate detection logic lives in the observer binary.
3. Empty-state handling: healthy scenarios produce either an empty-state line or the section is omitted — never a partial or malformed Section 9 header.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` focused tests — observer rendering tests covering non-empty, empty-state, and section-order cases.

### Commands

1. `cargo test -p worldwake-cli --bin observer` — targeted observer rendering tests.
2. `cargo test -p worldwake-cli` — full CLI/observer test suite.
3. `cargo clippy --workspace --all-targets -- -D warnings` — workspace lint (CI parity).
4. `cargo test --workspace` — full regression sweep.

## Outcome

Completed on 2026-04-18.

- Added live per-tick `SurvivalForensicExtractor` capture to `crates/worldwake-cli/src/bin/observer.rs`, reusing the observer's existing world, scheduler, decision-trace, and action-trace context rather than inventing a replay-only extraction path.
- Added `--critical-window-top-n <N>` to the observer CLI with default `3`; `0` now disables Section 9 entirely, while enabled healthy runs render the empty-state line `No authored-critical windows detected.`.
- Added `## Section 9 — Critical Window Forensics` rendering after Section 8, including per-window summaries for agent, need, tick span, authored threshold, peak value, selected-goal summary, selected plan source summary, typed exhaustion/blocker summaries, and a bounded frame table with local authoritative survival context.
- Added four focused in-bin tests in `observer.rs` covering populated Section 9 output, empty-state rendering, explicit disablement, and section-order continuity.

## Deviations

- The focused proof lives in `crates/worldwake-cli/src/bin/observer.rs` instead of a new `crates/worldwake-cli/tests/observer_critical_window_section.rs` file. Reassessment against the live CLI boundary showed the owned seam is the local `format_report(...)` surface, and keeping the tests in-bin avoided inventing duplicate report-construction scaffolding for a small single-binary change.
- `cargo test --workspace` remained truthful verification for this ticket, but the existing long-running `#[ignore]` survival scenarios in `golden_survival_baseline`, `golden_survival_scattered`, and `golden_survival_contested` still did not execute in that command. The command proved the workspace stayed green with those binaries compiled and their non-ignored tests passing.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
