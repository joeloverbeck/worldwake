# S117CONMAIOBS-001: Anomaly infrastructure and multi-agent rendering

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

The observer's `AnomalyKind` enum currently has six variants, and the `Anomaly` struct carries a single `agent_name: String` per entry. The four new S117 detectors (`GeographicConvergence`, `MaintenanceStarvation`, `RecipeMonoculture`, `AcuteNeedSpike`) cannot attach labels or multi-agent headers without infrastructure additions. This ticket lands the label-enum extensions and the outer-struct field so subsequent detector tickets have a stable surface to emit into.

## Assumption Reassessment (2026-04-18)

1. `AnomalyKind` is defined at `crates/worldwake-cli/src/bin/observer.rs:729` with `#[derive(Clone, Copy)]` and six existing unit variants (`RedundantPerception`, `ActionLoop`, `StuckAgent`, `FailedActionSpiral`, `SustainedCriticalNeed`, `UnaddressedNeed`). Adding unit variants preserves `Copy`. Its `label()` method lives in the same `impl` block and maps each variant to a SCREAMING_SNAKE_CASE string used in the Section 3 header.
2. `Anomaly` struct is defined at `crates/worldwake-cli/src/bin/observer.rs:722` with fields `kind`, `agent_name: String`, `description: String`, `tick_range: Option<(Tick, Tick)>`. There are six in-file construction sites (lines 775, 791, 831, 848, 917, 962) plus one renderer read at line 1699. All usage is confined to `bin/observer.rs`; no other crate imports the struct.
3. Shared abstraction boundary under audit: the `Anomaly` struct + `AnomalyKind::label()` + the single Section 3 render-path formatter at `bin/observer.rs:1696-1709`. This ticket does not cross a crate boundary.

## Architecture Check

1. Keeping `AnomalyKind` as a `Copy` label enum and attaching rich data to the outer `Anomaly` struct is the smaller-blast-radius choice: none of the six existing detectors need migration, and the existing render path stays a single formatter. The alternative — migrating `AnomalyKind` to a data-bearing enum — would drop `Copy`, force every existing variant to be restructured, and fan out across the six construction sites and any future sibling detectors.
2. `additional_agent_names: Option<Vec<String>>` is explicitly optional so existing detectors do not need to opt in; they simply set `None` and the renderer falls back to the single-agent header. No backward-compatibility shim is introduced — this is a greenfield field on a private struct.

## Verification Layers

1. New variants exist on `AnomalyKind` and render with the correct label → focused unit coverage (compile-time match exhaustiveness + `label()` tests if present; otherwise inline `assert_eq!` in a small unit test added in this ticket).
2. Existing Section 3 render output is byte-identical for single-agent anomalies (i.e., the `None` fallback preserves today's header format) → observer dump comparison against a known healthy scenario; covered by `test_observer_mode_simulation_runs` continuing to pass plus a focused unit test asserting the single-agent header branch.
3. Multi-agent header formats a comma-separated list when `additional_agent_names` is `Some(_)` → focused unit test on the render helper (extracted or invoked directly).
4. Single-layer ticket: no action trace, event-log, or decision-trace proof surface applies — this is a pure observer-tool data-structure extension.

## What to Change

### 1. Extend `AnomalyKind` with four unit variants

In `crates/worldwake-cli/src/bin/observer.rs`, add to the `enum AnomalyKind` (around line 729):

- `GeographicConvergence`
- `MaintenanceStarvation`
- `RecipeMonoculture`
- `AcuteNeedSpike`

Extend the `AnomalyKind::label()` match with:

- `Self::GeographicConvergence => "GEOGRAPHIC_CONVERGENCE"`
- `Self::MaintenanceStarvation => "MAINTENANCE_STARVATION"`
- `Self::RecipeMonoculture => "RECIPE_MONOCULTURE"`
- `Self::AcuteNeedSpike => "ACUTE_NEED_SPIKE"`

### 2. Extend the `Anomaly` struct

Add one field to the `struct Anomaly` definition (around line 722):

```rust
additional_agent_names: Option<Vec<String>>, // None for single-agent; Some(sorted_names) for multi-agent
```

Update the six existing `Anomaly { ... }` construction sites (lines 775, 791, 831, 848, 917, 962) by appending `additional_agent_names: None,` to each.

### 3. Update Section 3 render path for multi-agent header

At `bin/observer.rs:1696-1709`, replace the header format call with a branch:

- When `anomaly.additional_agent_names` is `None`: render `### Anomaly {} — {} ({})` exactly as today (`anomaly.agent_name`).
- When `Some(names)` with `names.len() > 0`: render `### Anomaly {} — {} ({}, {})` where the second argument is `anomaly.agent_name` and the third joins `names` with `", "`.

The body (`anomaly.description`) and tick-range lines remain unchanged.

### 4. Focused unit tests

Add a small `#[cfg(test)] mod tests` block (or extend the existing one at `bin/observer.rs:2792`) with:

- `test_anomaly_kind_label_emits_new_labels` — asserts each of the four new variants produces its expected label string.
- `test_anomaly_render_single_agent_header_unchanged` — constructs an `Anomaly` with `additional_agent_names: None` and asserts the header string matches `"### Anomaly 1 — REDUNDANT_PERCEPTION (Alice)"` or equivalent existing format.
- `test_anomaly_render_multi_agent_header` — constructs an `Anomaly` with `agent_name: "Alice"` and `additional_agent_names: Some(vec!["Bob".into(), "Carol".into()])` and asserts the header renders `"### Anomaly 1 — GEOGRAPHIC_CONVERGENCE (Alice, Bob, Carol)"`.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Any actual detector logic (covered in 002–005).
- Emitting anomalies with the new variants (covered in 002–005).
- Section 2 supplementary tables (covered in 006).
- Goldens that exercise end-to-end behavior (covered in 007).
- Extracting the observer into a `src/observer/` module tree — deferred to a separate refactor if needed.
- Adding a `medium(need)` helper to `DriveThresholds` — the detector tickets perform the per-need match locally.

## Acceptance Criteria

### Tests That Must Pass

1. New unit test `test_anomaly_kind_label_emits_new_labels` passes.
2. New unit test `test_anomaly_render_single_agent_header_unchanged` passes.
3. New unit test `test_anomaly_render_multi_agent_header` passes.
4. Existing integration test `test_observer_mode_simulation_runs` (`crates/worldwake-cli/tests/integration.rs`) continues to pass.
5. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. `AnomalyKind: Copy` is preserved — `#[derive(Clone, Copy)]` still applies.
2. Single-agent anomalies emit exactly the same Section 3 header bytes as before this ticket (the `None` branch is the backward-identity path).
3. No other crate imports `Anomaly` or `AnomalyKind` — both remain private to `bin/observer.rs`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (extended `#[cfg(test)] mod tests`) — three new unit tests above.

### Commands

1. `cargo test -p worldwake-cli --bin observer`
2. `cargo test -p worldwake-cli`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-18.

- Extended `crates/worldwake-cli/src/bin/observer.rs` with the four S117 anomaly labels, added `additional_agent_names: Option<Vec<String>>` to the private `Anomaly` carrier, and updated the Section 3 header formatting path to support multi-agent anomaly headers while preserving the single-agent bytes.
- Added focused observer unit coverage for the new labels plus single-agent and multi-agent header rendering.
- Kept the change local to `bin/observer.rs`; no other crate imports or APIs changed.

## Deviations

- The live observer file did not already expose a dedicated anomaly-header helper, so the render-path change was implemented by extracting a private same-file `format_anomaly_header()` helper to give the focused tests an honest formatter seam.
- Because this infrastructure ticket lands enum variants before the follow-up detector tickets construct them, the private `AnomalyKind` enum now carries a targeted `#[allow(dead_code)]` annotation to keep the required CI-matching clippy pass green without widening this ticket into detector implementation.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
