# S144AGGSCEDIA-006: Observer Section 13 renderer and CLI flags

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — observer binary tooling only (new render section + CLI flags)
**Deps**: archive/tickets/S144AGGSCEDIA-004.md, archive/tickets/S144AGGSCEDIA-005.md

## Problem

S144's `ScenarioDiagnosticsReport` needs a developer-facing surface. The observer binary renders 12 sections and 9 anomaly detectors but has no Section 13 for the aggregate diagnostics, and no CLI flags to choose text/JSON output, percentile sets, or top-N caps.

## Assumption Reassessment (2026-05-14)

1. `crates/worldwake-cli/src/bin/observer.rs` currently renders through Section 12 (Contention, ~line 3509); section render functions follow the `render_*_section` naming pattern (7 such functions); the `ObserverCli` struct (~line 42) parses flags via `clap` derive with `#[arg(...)]` attributes (existing flags: `--ticks`, `--output`, `--critical-window-top-n`, `--top-omissions`, `--contention-top-n`, `--ignore-lints`); the observer test module is at `#[cfg(test)]` ~line 4937. Section 13 is the next free section number.
2. S144 spec D6+D7 (`specs/S144-aggregate-scenario-diagnostics.md`) specify `render_scenario_diagnostics_section(report, format, out)`, a `DiagnosticsFormat` enum (`text`|`json`), and four flags: `--diagnostics-format`, `--diagnostics-percentiles`, `--diagnostics-top-n`, `--no-diagnostics` (default on). The renderer consumes `ScenarioDiagnosticsReport` (ticket 004, archived at `archive/tickets/S144AGGSCEDIA-004.md`) and is fed by `build_scenario_diagnostics` (ticket 005).
3. Ticket 004's reassessment proved the report type is format-agnostic serde data, but payload-bearing `GoalKind` / `Discrepancy` map keys cannot be treated as raw JSON object keys in every report. This ticket owns the deterministic observer JSON representation for those maps.
4. Mixed-layer shared boundary under audit: this ticket consumes the `ScenarioDiagnosticsReport` type (ticket 004) and the `build_scenario_diagnostics` function (ticket 005); the contract under audit is the observer's section-rendering convention — the new section must follow the existing `render_*_section` pattern and section-numbering so the dump format stays consistent.

## Architecture Check

1. Adding Section 13 alongside the existing 12 sections and the anomaly channel keeps the observer a pure read-only consumer — it calls `build_scenario_diagnostics` and renders, never mutating world state. The JSON format emits a deterministic report representation that preserves the `ScenarioDiagnosticsReport` data without relying on raw JSON object keys for payload-bearing enum maps.
2. No backwards-compatibility aliasing/shims — the new flags and renderer are additive; `--no-diagnostics` provides opt-out without a legacy code path.

## Verification Layers

1. Text rendering produces a well-formed Section 13 (headers/tables match the `render_*_section` convention) -> focused unit test in the observer test module.
2. JSON format emits the report such that it parses back to an identical `ScenarioDiagnosticsReport`, including payload-bearing map keys -> focused unit test (serialize via renderer -> deserialize -> equal).
3. `--diagnostics-top-n` caps rendered map entries (N entries + "...others") -> focused unit test.
4. Single binary-tooling ticket: proof surfaces are headless render tests and command-built smoke checks on the CLI surface; there is no engine decision/action/event layer because the observer only reads.

## What to Change

### 1. `DiagnosticsFormat` enum + CLI flags

Add `DiagnosticsFormat { Text, Json }` and four `#[arg(...)]` fields to `ObserverCli`: `--diagnostics-format` (default `text`), `--diagnostics-percentiles` (override the percentile set), `--diagnostics-top-n` (cap rendered entries), `--no-diagnostics` (opt out; diagnostics on by default).

### 2. `render_scenario_diagnostics_section`

Add `render_scenario_diagnostics_section(report: &ScenarioDiagnosticsReport, format: DiagnosticsFormat, out: &mut impl Write) -> io::Result<()>` following the existing `render_*_section` pattern. Text format renders tables with top-N capping + "...others" summary; JSON format emits a deterministic representation that round-trips through the renderer for the full report, including payload-bearing enum map keys.

### 3. Wire Section 13 into the observer dump

Call `build_scenario_diagnostics` over the run's accumulated traces + event log and invoke the renderer as Section 13, gated by `--no-diagnostics`.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — `ObserverCli` flags, `DiagnosticsFormat`, renderer, Section 13 wiring)

## Out of Scope

- The aggregator itself — ticket 005.
- Golden / fixture coverage — ticket 007.
- Periodic snapshot mode — the observer remains single-shot.
- Live metrics dashboard — S144 is post-run / on-demand only.

## Acceptance Criteria

### Tests That Must Pass

1. `render_scenario_diagnostics_section` in text format produces a Section 13 consistent with the existing `render_*_section` convention.
2. JSON format output parses back to an identical `ScenarioDiagnosticsReport`, including payload-bearing enum map keys.
3. `--diagnostics-top-n N` caps rendered map entries at N plus a "...others" summary line.
4. `--no-diagnostics` suppresses Section 13 entirely.
5. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. The observer remains a pure read-only consumer — Section 13 mutates no world state.
2. JSON output preserves the full report data through the observer's deterministic JSON representation — round-tripping it yields an equal `ScenarioDiagnosticsReport`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (inline `#[cfg(test)]`) — text-format Section 13 rendering; JSON round-trip; `--diagnostics-top-n` capping; `--no-diagnostics` suppression.

### Commands

1. `cargo test -p worldwake-cli observer`
2. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
3. `cargo build -p worldwake-cli --bin observer` (confirms the binary builds with the new flags)
