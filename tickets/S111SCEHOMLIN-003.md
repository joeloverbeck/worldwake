# S111SCEHOMLIN-003: Wire lints into scenario load + override mechanism

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S111SCEHOMLIN-002

## Problem

S111SCEHOMLIN-002 builds the `scenario::lints` module but it is not yet enforced — `spawn_scenario` proceeds even on lint failures, and there is no opt-out mechanism for scenarios that legitimately need uniform populations (regression tests targeting a specific behavior). This ticket wires lints into the load path, adds the scenario-level override mechanism, and adds the `--ignore-lints` CLI escape hatch with a visible warning.

## Assumption Reassessment (2026-04-19)

1. **Existing tests on `spawn_scenario`**: `crates/worldwake-cli/tests/integration.rs` and `crates/worldwake-cli/tests/golden_observer_anomalies.rs` exercise the scenario-load path. Both call `spawn_scenario` (via `Grep` in `crates/worldwake-cli/src` and `tests/`). After this ticket, both will run lints; their committed scenarios must already pass lints (verified in S111SCEHOMLIN-004) or have explicit overrides.
2. **Spec/docs reference**: `specs/S111-scenario-homogeneity-lints.md` D4 + D6.4 + D6.5 (current revision after `/reassess-spec` 2026-04-19).
3. **Shared abstraction boundary**: `spawn_scenario` (`crates/worldwake-cli/src/scenario/mod.rs:105`) is the canonical scenario-bootstrap entry point. All bins (`main.rs`, `bin/observer.rs`) and tests funnel through it. Wiring lints here is the single-point enforcement boundary.
6. **AI regression layer**: N/A — no AI behavior changes; lints run before any tick advances.
13. **Adjacent contradictions**: `ScenarioError::Validation(String)` is currently the only "soft" error variant (see `crates/worldwake-cli/src/scenario/mod.rs:43-48`). Reusing it for empty-justification rejection (rather than adding a separate variant) keeps the error taxonomy minimal — the existing `Display` impl at line 1166 will format the message naturally.

## Architecture Check

1. The override mechanism lives on `ScenarioDef` (RON-deserializable, scenario-author-visible) rather than in CLI flags or environment variables. This makes the intent legible inside the scenario file itself: future readers can see exactly which lints were silenced and why. The required-non-empty justification string makes silent dismissal impossible.
2. `--ignore-lints` exists for ad-hoc debug runs only, emits a stderr warning naming every suppressed rule, and is a CLI-bin-level concern (not a `ScenarioDef` field). This separation keeps committed scenarios honest while preserving an escape hatch for one-off experimentation.
3. No backwards-compatibility shims. The new field on `ScenarioDef` uses `#[serde(default)]` so existing scenario RON files without the field continue to deserialize, but this is forward-compat (new field, old data), not a deprecated alias.

## Verification Layers

1. `spawn_scenario` rejects scenarios with unsuppressed lint failures -> focused unit test asserting `Err(ScenarioError::LintFailure(_))` on a synthetic homogeneous scenario.
2. `scenario_lint_overrides` with non-empty justification suppresses matching rules -> focused unit test (D6.4).
3. `scenario_lint_overrides` with empty justification returns `ScenarioError::Validation` -> focused unit test (D6.5).
4. `--ignore-lints` flag suppresses all failures and emits a stderr warning -> CLI-level smoke test (manual or scripted via `assert_cmd`).
5. Single-layer scope: load-time validation only; no decision-trace, action-trace, or event-log layers are involved.

## What to Change

### 1. Add `scenario_lint_overrides` field to `ScenarioDef`

In `crates/worldwake-cli/src/scenario/types.rs`, add to `ScenarioDef` (after the existing `compaction_interval` field at line 43):

```rust
/// Per-rule lint overrides keyed by `LintRule` variant. The string value is a
/// required justification (empty strings are rejected with ScenarioError::Validation).
#[serde(default)]
pub scenario_lint_overrides: BTreeMap<crate::scenario::lints::LintRule, String>,
```

Add the necessary `use std::collections::BTreeMap;` at the top if not already present (the file imports `std::num::NonZeroU32` but not `BTreeMap` — verify and add).

### 2. Add `LintFailure` variant to `ScenarioError`

In `crates/worldwake-cli/src/scenario/mod.rs:43-48`, extend `ScenarioError`:

```rust
pub enum ScenarioError {
    Io(std::io::Error),
    Parse(ron::error::SpannedError),
    Validation(String),
    World(worldwake_core::WorldError),
    LintFailure(lints::LintReport),
}
```

Update the `Display` impl at line 68 to format `LintFailure` — list each failure's rule + affected agents + detail on its own line, prefixed with `lint failure:`.

### 3. Add `lints::filter_overrides` helper

In `crates/worldwake-cli/src/scenario/lints.rs`:

```rust
pub fn filter_overrides(
    mut report: LintReport,
    overrides: &BTreeMap<LintRule, String>,
) -> Result<LintReport, super::ScenarioError> {
    for (rule, justification) in overrides {
        if justification.trim().is_empty() {
            return Err(super::ScenarioError::Validation(format!(
                "lint override for {rule:?} requires a non-empty justification string"
            )));
        }
    }
    let suppressed: BTreeSet<LintRule> = overrides.keys().copied().collect();
    report.failures.retain(|f| !suppressed.contains(&f.rule));
    Ok(report)
}
```

This rejects empty justifications first (returns `Err`), then filters out failures whose rule is in the override set.

### 4. Wire `run_lints` + `filter_overrides` into `spawn_scenario`

In `crates/worldwake-cli/src/scenario/mod.rs:105-143`, at the very top of `spawn_scenario` (before `let mut names: BTreeMap<...> = BTreeMap::new();` at line 106):

```rust
pub fn spawn_scenario(def: &ScenarioDef) -> Result<SpawnedSimulation, ScenarioError> {
    let report = lints::run_lints(def);
    let report = lints::filter_overrides(report, &def.scenario_lint_overrides)?;
    if !report.failures.is_empty() {
        return Err(ScenarioError::LintFailure(report));
    }
    // existing body (let mut names = ...; ...)
```

`load_scenario_file` (line 91) is unchanged — it remains a pure deserialize. Lints fire at spawn time, which is the boundary every entry point (`main.rs`, `bin/observer.rs`, integration tests) crosses.

### 5. Add `--ignore-lints` flag to CLI bins

In `crates/worldwake-cli/src/main.rs` and `crates/worldwake-cli/src/bin/observer.rs`, add a `--ignore-lints` flag that, when present, calls a new helper `lints::run_lints_for_warning(def)` (or similar) that emits each failure as a stderr warning prefixed `WARNING (lint suppressed by --ignore-lints):`, then bypasses the `ScenarioError::LintFailure` short-circuit by either:

(a) clearing `def.scenario_lint_overrides`-equivalent before calling `spawn_scenario` is not viable because `def` is an immutable `&ScenarioDef`; instead,

(b) introduce a sibling entry point `spawn_scenario_ignoring_lints(def: &ScenarioDef) -> Result<SpawnedSimulation, ScenarioError>` that runs the body of `spawn_scenario` with the lint check skipped (still emits `lints::run_lints(def)` failures to stderr for visibility).

Use approach (b) — it makes the bypass explicit at the call site and avoids mutating `ScenarioDef`.

### 6. Add unit tests in `lints.rs` (D6.4 + D6.5)

Add to the existing `#[cfg(test)] mod tests` from S111SCEHOMLIN-002:

1. `override_with_justification_suppresses_failure` (D6.4) — homogeneous scenario + override map `{ ProfileHomogeneity: "covers identical-twin regression" }` → `spawn_scenario` returns `Ok(_)` (or, more narrowly, `filter_overrides` strips the failure).
2. `override_with_empty_justification_returns_validation_error` (D6.5) — homogeneous scenario + `{ ProfileHomogeneity: "" }` → `spawn_scenario` returns `Err(ScenarioError::Validation(_))` whose message names the rule.
3. `unsuppressed_failure_short_circuits_spawn` — homogeneous scenario, no override → `spawn_scenario` returns `Err(ScenarioError::LintFailure(_))` containing the `ProfileHomogeneity` failure.

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — add `scenario_lint_overrides` field + `BTreeMap` import)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — add `LintFailure` variant, update `Display`, wire `run_lints` into `spawn_scenario`, add `spawn_scenario_ignoring_lints`)
- `crates/worldwake-cli/src/scenario/lints.rs` (modify — add `filter_overrides`, add 3 new unit tests)
- `crates/worldwake-cli/src/main.rs` (modify — `--ignore-lints` flag wiring)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — `--ignore-lints` flag wiring)

## Out of Scope

- The two lint rules themselves (delivered by S111SCEHOMLIN-002).
- The PlanningSnapshot doctest (S111SCEHOMLIN-001).
- Sweeping `scenarios/*.ron` and adding overrides where needed (S111SCEHOMLIN-004).
- Adding new lint rules beyond `ProfileHomogeneity` and `UnreachableExplorationDrive`.
- Refactoring the existing `ScenarioError` taxonomy or `Display` impl beyond the new variant.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli scenario::lints` (the 3 new tests pass + the 6 from S111SCEHOMLIN-002 still pass).
2. `cargo test -p worldwake-cli` (no regression in existing CLI tests, including `tests/integration.rs` and `tests/golden_observer_anomalies.rs` — these may need scenario fixes from S111SCEHOMLIN-004 first; if so, this ticket's PR should land after 004 or include the minimal fix).
3. Manual smoke: `cargo run --bin worldwake-cli -- --scenario <homogeneous-scenario>` returns nonzero exit + lint-failure stderr; rerun with `--ignore-lints` returns zero exit + warning stderr.
4. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. `spawn_scenario` rejects every scenario whose `LintReport` has unsuppressed `failures`.
2. `scenario_lint_overrides` entries with empty/whitespace-only justification strings are always rejected.
3. `--ignore-lints` is the only way to bypass a lint failure without an in-scenario override; it always emits a stderr warning naming each suppressed rule.
4. `load_scenario_file` remains a pure deserialize — it never invokes lints.
5. New `scenario_lint_overrides` field uses `#[serde(default)]` so existing scenario RON files without the field continue to deserialize.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/lints.rs` — 3 new unit tests covering D6.4, D6.5, and the `spawn_scenario` short-circuit. Rationale: validates the override filter, the empty-justification rejection, and the integration boundary.
2. Existing `crates/worldwake-cli/tests/integration.rs` and `tests/golden_observer_anomalies.rs` — re-run unchanged; if they fail because their committed scenarios now violate lints, the fix belongs in S111SCEHOMLIN-004, not here.

### Commands

1. `cargo test -p worldwake-cli scenario::lints` (targeted — new + carried-over tests)
2. `cargo test -p worldwake-cli` (full crate regression including integration tests)
3. `cargo clippy --workspace --all-targets -- -D warnings` (CI parity)
