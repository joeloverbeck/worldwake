# S120SURCRIWIN-003: Observer Section 9 — Critical Window Forensics rendering

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None — this ticket adds an optional rendering section to the observer binary; no simulation, planner, or agent-decision behavior changes. The observer itself is a passive read-only tool (FND-26).
**Deps**: `S120SURCRIWIN-001`

## Problem

The spec marks observer integration as optional for the first landing of S120 (`specs/S120-survival-critical-window-forensics.md` D6: "Observer integration is optional for the first landing if the shared golden-support surface is sufficient"). This ticket closes D6 so that a scenario-analysis session — driven by `/scenario-analysis` or the manual observer dump pipeline — can drill into top-N longest authored-critical windows without re-implementing the forensic helper in a different consumer.

The observer binary today ends at `## Section 8 — Budget Exhaustion Snapshots` (`crates/worldwake-cli/src/bin/observer.rs:349,2111`). Adding Section 9 is the natural next-integer extension and follows the established `## Section N — Title` convention enforced by `/scenario-analysis`'s anomaly-parsing conventions.

## Assumption Reassessment (2026-04-18)

1. Observer binary structure:
   - `crates/worldwake-cli/src/bin/observer.rs` — existing sections 1 through 8 render via `writeln!(out, "## Section N — ...")` markers at lines 1278, 1304, 1451, 1473, 1640, 1744, 1817, and 349 (Section 8). The file ends around line 2111+ after Section 8's budget-exhaustion rendering. Validated during `/reassess-spec` pass on 2026-04-18.
   - `worldwake-cli`'s `Cargo.toml` already depends on `worldwake-ai`, so `worldwake-ai::survival_forensics` is importable after `S120SURCRIWIN-001` lands.
2. Spec reference: `specs/S120-survival-critical-window-forensics.md` D6 (lines 183–191) specifies `## Section 9 — Critical Window Forensics`. The reassessment confirmed section numbering follows sequential integers (1, 2, 3, ..., 8) — the original spec draft's "Section 2.5" suggestion was corrected to "Section 9" during reassessment per output-format-fidelity rules.
3. Shared abstraction boundary: the observer consumes the same `CriticalWindowReport` type that golden goldens consume. The extraction path differs (observer replays from the saved event-log dump; goldens observe per-tick live), but the report model is identical — this is exactly the dual-use pattern the module placement in `S120SURCRIWIN-001` was designed for.
13. Adjacent contradiction audit: S117's observer Section 3 anomaly detectors are independent of this section. S120 Section 9 renders forensic drill-down for top-N longest authored-critical windows; S117 Section 3 renders anomaly detections. Both may render in the same observer dump without coupling. This is the in-scope consequence captured in the spec's Dependencies section.

## Architecture Check

1. Keeping the section optional (renders only when requested, and only for top-N longest windows per the spec) preserves observer readability for scenarios without survival distress. Every other existing section is unconditional; Section 9 being opt-in is a deliberate scale choice.
2. Consuming the runtime `survival_forensics` module rather than re-implementing extraction in the observer binary preserves one canonical extraction path — deleting Section 9 does not weaken the goldens' surface and vice versa.
3. Deferring this work until after `S120SURCRIWIN-001` and `S120SURCRIWIN-002` prove the surface in goldens is intentional: the observer section is downstream of the canonical surface, not its own source of truth.

## Verification Layers

1. Section 9 renders correctly for a scenario with at least one authored-critical window → focused observer test constructing a minimal event-log dump with a synthetic window and asserting the Section 9 output matches an expected snippet.
2. Section 9 is suppressed (zero output beyond an empty-state line, or entirely omitted per spec) for a healthy scenario → focused observer test with no authored-critical windows.
3. Section numbering continuity → file-level grep assertion that Section 9 follows Section 8 in source order. No cross-layer mapping applies — observer is a pure read-only rendering surface.

## What to Change

### 1. Extract observer-side window reports

Add a helper at the observer side (local to the `observer.rs` binary or in a new `crates/worldwake-cli/src/observer/survival_forensics_view.rs` module) that replays the event-log dump into the same `SurvivalForensicExtractor::observe` calls the goldens use. Reuse the runtime extractor; do not re-implement detection.

### 2. Section 9 rendering

After the Section 8 rendering block (around line 2111+), add:

```
## Section 9 — Critical Window Forensics

<per-agent top-N windows rendered as markdown subsections>
```

Each rendered window includes: agent name, need, `start_tick`..`end_tick`, authored threshold, peak value, selected-goal summary across captured frames, typed exhaustion state, typed blocker summary, and local authoritative summary at key frames.

Top-N is configurable via an observer command-line flag (e.g., `--critical-window-top-n <N>`, default `3`). If no authored-critical windows exist, render a single empty-state line: `No authored-critical windows detected.`

### 3. Observer-level focused tests

Add `crates/worldwake-cli/tests/observer_critical_window_section.rs` covering:
- Section 9 renders for a synthetic dump containing one window (assert expected header + content)
- Section 9 renders empty-state for a healthy dump
- Section numbering sequence: Section 8 header appears before Section 9 header in the output

### 4. Scenario-analysis integration note (not code)

Not in this ticket — documentation of the new section's consumption contract by `/scenario-analysis` is deferred to `S120SURCRIWIN-004`.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — add Section 9 rendering block after Section 8)
- `crates/worldwake-cli/src/observer/survival_forensics_view.rs` (new, optional — may be inlined into `observer.rs` if small)
- `crates/worldwake-cli/tests/observer_critical_window_section.rs` (new)

## Out of Scope

- Changes to `worldwake-ai::survival_forensics` module internals — consume it as-is from `S120SURCRIWIN-001`.
- Changes to goldens or their harness — see `S120SURCRIWIN-002`.
- Documentation updates — see `S120SURCRIWIN-004`.
- Integration with S117 observer anomaly detectors. Section 9 is independent of Section 3; cross-reference between the two is future work.
- Making Section 9 render unconditionally. Per spec, it renders only when windows exist or when explicitly requested.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli --test observer_critical_window_section` — focused observer test suite passes.
2. Existing observer regression tests (if any): `cargo test -p worldwake-cli` remains green.
3. Existing suite: `cargo test --workspace` remains green.
4. Lint: `cargo clippy --workspace --all-targets -- -D warnings` passes.

### Invariants

1. Section numbering continuity: `## Section 9 — Critical Window Forensics` appears after `## Section 8` in observer source order and in rendered output.
2. Section 9 renders from the `worldwake-ai::survival_forensics` module (canonical extractor); no duplicate detection logic lives in the observer binary.
3. Empty-state handling: healthy scenarios produce either an empty-state line or the section is omitted — never a partial or malformed Section 9 header.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/tests/observer_critical_window_section.rs` — focused observer rendering tests covering non-empty, empty-state, and section-order cases.

### Commands

1. `cargo test -p worldwake-cli --test observer_critical_window_section` — targeted observer rendering tests.
2. `cargo test -p worldwake-cli` — full CLI/observer test suite.
3. `cargo clippy --workspace --all-targets -- -D warnings` — workspace lint (CI parity).
4. `cargo test --workspace` — full regression sweep.
