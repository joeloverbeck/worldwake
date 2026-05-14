# S144AGGSCEDIA-006: Observer Section 13 renderer and CLI flags

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — observer binary tooling only (new render section + CLI flags)
**Deps**: archive/tickets/S144AGGSCEDIA-004.md, archive/tickets/S144AGGSCEDIA-005.md

## Problem

S144's `ScenarioDiagnosticsReport` needs a developer-facing surface. The observer binary renders 12 sections and 9 anomaly detectors but has no Section 13 for the aggregate diagnostics, and no CLI flags to choose text/JSON output, percentile sets, or top-N caps.

## Assumption Reassessment (2026-05-14)

1. Before this ticket, `crates/worldwake-cli/src/bin/observer.rs` rendered through Section 12 (Contention); section render functions followed the `render_*_section` naming pattern; the `ObserverCli` struct parsed flags via `clap` derive with `#[arg(...)]` attributes (existing flags: `--ticks`, `--output`, `--critical-window-top-n`, `--top-omissions`, `--contention-top-n`, `--ignore-lints`); the observer test module already lived inline under `#[cfg(test)]`. Section 13 was the next free section number.
2. S144 spec D6+D7 (`archive/specs/S144-aggregate-scenario-diagnostics.md`) specify `render_scenario_diagnostics_section(report, format, out)`, a `DiagnosticsFormat` enum (`text`|`json`), and four flags: `--diagnostics-format`, `--diagnostics-percentiles`, `--diagnostics-top-n`, `--no-diagnostics` (default on). The renderer consumes `ScenarioDiagnosticsReport` (ticket 004, archived at `archive/tickets/S144AGGSCEDIA-004.md`) and is fed by `build_scenario_diagnostics` (ticket 005).
3. Ticket 004's reassessment proved the report type is format-agnostic serde data, but payload-bearing `GoalKind` / `Discrepancy` map keys cannot be treated as raw JSON object keys in every report. This ticket owns the deterministic observer JSON representation for those maps.
4. Mixed-layer shared boundary under audit: this ticket consumes the `ScenarioDiagnosticsReport` type (ticket 004) and the `build_scenario_diagnostics` function (ticket 005); the contract under audit is the observer's section-rendering convention — the new section must follow the existing `render_*_section` pattern and section-numbering so the dump format stays consistent.

## Architecture Check

1. Adding Section 13 alongside the existing 12 sections and the anomaly channel keeps the observer a pure read-only consumer — it calls `build_scenario_diagnostics` and renders, never mutating world state. The JSON format emits a deterministic report representation that preserves the `ScenarioDiagnosticsReport` data without relying on raw JSON object keys for payload-bearing enum maps.
2. No backwards-compatibility aliasing/shims — the new flags and renderer are additive; `--no-diagnostics` provides opt-out without a legacy code path.

## Verified Layers

1. Text rendering produces a well-formed Section 13 (headers/tables match the `render_*_section` convention) -> focused unit test in the observer test module.
2. JSON format emits the report such that it parses back to an identical `ScenarioDiagnosticsReport`, including payload-bearing map keys -> focused unit test (serialize via renderer -> deserialize -> equal).
3. `--diagnostics-top-n` caps rendered map entries (N entries + "...others") -> focused unit test.
4. `--no-diagnostics` suppresses Section 13 entirely -> focused unit test through `format_report`'s optional diagnostics report.
5. Single binary-tooling ticket: proof surfaces are headless render tests and command-built smoke checks on the CLI surface; there is no engine decision/action/event layer because the observer only reads.

## Landed Changes

### 1. `DiagnosticsFormat` enum + CLI flags

Added `DiagnosticsFormat { Text, Json }` and four `#[arg(...)]` fields to `ObserverCli`: `--diagnostics-format` (default `text`), `--diagnostics-percentiles` (comma-delimited percentile columns for text output), `--diagnostics-top-n` (cap rendered entries), and `--no-diagnostics` (opt out; diagnostics on by default).

### 2. `render_scenario_diagnostics_section`

Added `render_scenario_diagnostics_section(report, options, out)` following the existing `render_*_section` pattern. Text format renders Section 13 tables with top-N capping + "...others" summary. JSON format emits an observer-owned deterministic representation using array entries for report maps, so payload-bearing `GoalKind` and `Discrepancy` keys round-trip without relying on JSON object keys.

### 3. Wire Section 13 into the observer dump

The observer now calls `build_scenario_diagnostics` over the run's accumulated decision traces, extracted plan traces, repair traces, and event log, then invokes the renderer after Section 12 when diagnostics are enabled.

## Landed Files

- `crates/worldwake-cli/src/bin/observer.rs` (modified — `ObserverCli` flags, `DiagnosticsFormat`, deterministic JSON representation, Section 13 renderer, Section 13 wiring, focused tests)
- `crates/worldwake-cli/Cargo.toml` (modified — added `serde_json` for observer-owned deterministic JSON output)
- `Cargo.lock` (modified — direct `worldwake-cli` dependency edge for `serde_json`)

## Out of Scope

- The aggregator itself — ticket 005.
- Golden / fixture coverage — ticket 007.
- Periodic snapshot mode — the observer remains single-shot.
- Live metrics dashboard — S144 is post-run / on-demand only.

## Acceptance Result

### Completed Test Assertions

1. Passed: `render_scenario_diagnostics_section` in text format produces a Section 13 consistent with the existing `render_*_section` convention.
2. Passed: JSON format output parses back to an identical `ScenarioDiagnosticsReport`, including payload-bearing enum map keys.
3. Passed: `--diagnostics-top-n N` caps rendered map entries at N plus a "...others" summary line.
4. Passed: `--no-diagnostics` suppresses Section 13 entirely.
5. Passed: existing suite `cargo test -p worldwake-cli`.

### Invariants

1. The observer remains a pure read-only consumer — Section 13 mutates no world state.
2. JSON output preserves the full report data through the observer's deterministic JSON representation — round-tripping it yields an equal `ScenarioDiagnosticsReport`.

## Test Plan Result

### Added Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (inline `#[cfg(test)]`) — text-format Section 13 rendering; JSON round-trip through payload-bearing map keys; `--diagnostics-top-n` capping; `--no-diagnostics` suppression through the optional report handoff; CLI parse coverage for the new flags.

### Commands Passed

1. Passed `cargo test -p worldwake-cli --bin observer render_scenario_diagnostics_section`.
2. Passed `cargo test -p worldwake-cli --bin observer observer_cli_parses_top_omissions_default_and_override`.
3. Passed `cargo test -p worldwake-cli --bin observer format_report_includes_or_suppresses_section_13_from_report_option`.
4. Passed `cargo test -p worldwake-cli`.
5. Passed `cargo build -p worldwake-cli --bin observer`.
6. Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`.

## Outcome

Completed on 2026-05-14.

- Added observer Section 13 for aggregate scenario diagnostics in text mode.
- Added deterministic observer JSON output for `ScenarioDiagnosticsReport` that preserves payload-bearing map keys by encoding map entries as arrays.
- Added `--diagnostics-format`, `--diagnostics-percentiles`, `--diagnostics-top-n`, and `--no-diagnostics` to the observer CLI.
- Wired the observer's accumulated decision, plan, and repair traces into `build_scenario_diagnostics` without adding engine mutation or new authoritative state.

## Deviations

- The landed renderer takes `DiagnosticsRenderOptions` rather than the spec sketch's separate `format` argument so the same call carries text percentile columns and top-N caps.
- `serde_json` was added to `worldwake-cli` because this ticket owns the observer JSON representation rather than relying on the `worldwake-ai` type module's dev-only JSON smoke dependency.

## Verification Result

- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-cli --bin observer -- --list` to resolve bin-local test selectors.
- Passed `cargo test -p worldwake-cli --bin observer render_scenario_diagnostics_section`.
- Passed `cargo test -p worldwake-cli --bin observer observer_cli_parses_top_omissions_default_and_override`.
- Passed `cargo test -p worldwake-cli --bin observer format_report_includes_or_suppresses_section_13_from_report_option`.
- Passed `cargo test -p worldwake-cli`.
- Passed `cargo build -p worldwake-cli --bin observer`.
- Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`.
- Passed `python3 .codex/skills/implement-ticket/scripts/check_closeout.py tickets/S144AGGSCEDIA-006.md`.
- Passed `git diff --check -- Cargo.lock crates/worldwake-cli/Cargo.toml crates/worldwake-cli/src/bin/observer.rs tickets/S144AGGSCEDIA-006.md`.
