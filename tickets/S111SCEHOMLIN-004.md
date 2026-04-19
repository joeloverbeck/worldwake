# S111SCEHOMLIN-004: CI scenario sweep + fix existing scenarios

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S111SCEHOMLIN-003

## Problem

After S111SCEHOMLIN-003 wires lints into `spawn_scenario`, every committed scenario in `scenarios/` (and every scenario reached by integration/golden tests) starts running through `lints::run_lints`. If any committed scenario currently fails, every test that calls `spawn_scenario` on it begins to fail. This ticket sweeps the existing scenarios, fixes those that legitimately need varied profiles, and adds justified `scenario_lint_overrides` entries on those that legitimately need uniform populations (e.g., regression-targeted scenarios). It also adds the CI test that iterates `scenarios/*.ron` and asserts every committed scenario passes lints.

## Assumption Reassessment (2026-04-19)

1. **Existing scenarios in `scenarios/`** — verified at /reassess-spec time:
   - `survival-baseline.ron`
   - `survival-scattered.ron`
   - `survival-contested.ron`
   - `cli-evaluation.ron`
   - `drive-escalation-wash-priority.ron`
   `cli-evaluation.ron` already has heavily varied per-agent profiles (per its update history at lines 7-45). The survival/drive-escalation scenarios may or may not vary — the audit step below will determine which need fixes vs. overrides.
2. **Spec/docs reference**: `specs/S111-scenario-homogeneity-lints.md` D5 + D6.6 (current revision after `/reassess-spec` 2026-04-19).
3. **Shared abstraction boundary**: this ticket asserts that the contract from S111SCEHOMLIN-002/003 (no scenario passes load with unsuppressed lint failures) holds for every committed scenario at the time of landing. The CI test enforces it on every PR thereafter.
12. **Scenario isolation precision**: scenarios that legitimately need homogeneity (e.g., regression tests where every agent must behave identically to isolate one variable) get a `scenario_lint_overrides` entry whose justification names the regression invariant being preserved. Scenarios where homogeneity is incidental (just author oversight) get fixed by varying at least one profile field per FND-22.
13. **Adjacent contradictions**: if the audit reveals that `survival-baseline.ron` or another scenario has been silently riding herd-behavior since landing, that's a finding worth noting in the ticket completion summary — but the fix is to vary the agents (FND-22 alignment), not to silence the lint.

## Architecture Check

1. The CI test iterates `scenarios/*.ron` at runtime via `std::fs::read_dir`, calls `load_scenario_file` then `lints::run_lints`, and asserts every scenario has either zero failures or every failure is covered by an override with a non-empty justification. This is the canonical regression guard: a future PR adding a new scenario without varying its agents fails CI.
2. No backwards-compatibility shims. Scenarios that need overrides get them as authored fields; no silencing-by-flag.
3. The audit-and-fix pass is a one-time correctness pass for the existing scenario corpus — scenarios needing fixes get FND-22-aligned variations (different `courage`, different `perception_profile.observation_fidelity`, etc.), not synthetic distinctions.

## Verification Layers

1. Every committed `scenarios/*.ron` file passes `run_lints` (or has overrides with valid justifications) -> CI integration test iterating the directory.
2. Single-layer scope: this is a static repository invariant, asserted at test time. No simulation runtime layer is involved.

## What to Change

### 1. Audit each committed scenario

For each file in `scenarios/`, run `lints::run_lints` (manually or via a one-off script) and record the result. Expected outcomes per scenario (to confirm during implementation):

- `cli-evaluation.ron` — likely passes; profiles are heavily varied per the file's update history.
- `survival-baseline.ron` — needs verification; if all 3 agents share defaults, fix by varying.
- `survival-scattered.ron` — needs verification.
- `survival-contested.ron` — needs verification.
- `drive-escalation-wash-priority.ron` — likely homogeneous (regression-targeted); add override with justification.

For each scenario, decide: **fix** (vary an FND-22 axis) or **override** (justification names the regression invariant preserved by homogeneity). Default to fix; only override when homogeneity is the test contract.

### 2. Apply fixes / overrides in scenario files

For scenarios needing fixes: vary one or more fields per agent. Recommended axes (already present in `AgentDef`):
- `utility_profile.courage`, `social_weight`, `enterprise_weight`
- `perception_profile.observation_fidelity`, `entity_activation_threshold`
- `cognitive_profile.max_plan_depth`, `landmark_extraction_depth`

For scenarios needing overrides: add at the scenario root (after the existing fields):

```ron
scenario_lint_overrides: {
    ProfileHomogeneity: "drive-escalation regression: every agent must share identical thresholds so the failure mode under test isolates to wash-vs-water competition",
},
```

The justification must concretely name what the homogeneity preserves — generic strings like "test scenario" are insufficient (the lint cannot enforce content quality, but reviewer judgment will flag thin justifications).

### 3. Add the CI sweep test

New test file `crates/worldwake-cli/tests/scenario_lint_sweep.rs`:

```rust
//! CI guard: every committed scenario in scenarios/ must pass lints
//! (with explicit overrides where homogeneity is intentional).

use std::path::PathBuf;
use worldwake_cli::scenario::{lints, load_scenario_file};

#[test]
fn every_committed_scenario_passes_lints() {
    let scenarios_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios");
    let mut failed = Vec::new();

    for entry in std::fs::read_dir(&scenarios_dir).expect("scenarios dir readable") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("ron") {
            continue;
        }
        let def = load_scenario_file(&path).expect(&format!("load {path:?}"));
        let report = lints::run_lints(&def);
        let report = lints::filter_overrides(report, &def.scenario_lint_overrides)
            .expect(&format!("override justifications valid for {path:?}"));
        if !report.failures.is_empty() {
            failed.push((path.clone(), report.failures.clone()));
        }
    }

    assert!(
        failed.is_empty(),
        "scenarios with unsuppressed lint failures: {failed:#?}",
    );
}
```

The test reads `scenarios/` relative to the crate manifest. If the directory or any scenario file is added/removed/renamed, the test picks it up automatically.

### 4. Verify required `pub` exports

`load_scenario_file`, `lints` module (with `run_lints` + `filter_overrides`) must be reachable from `crates/worldwake-cli/tests/`. Confirm `crates/worldwake-cli/src/lib.rs` (or whatever exports the public API) re-exports `scenario` as a public module. If it doesn't today, add the re-export as part of this ticket — it's a side-effect of moving the test out of the `src/` tree.

## Files to Touch

- `scenarios/survival-baseline.ron` (modify, only if audit shows lint failure)
- `scenarios/survival-scattered.ron` (modify, only if audit shows lint failure)
- `scenarios/survival-contested.ron` (modify, only if audit shows lint failure)
- `scenarios/cli-evaluation.ron` (modify, only if audit shows lint failure)
- `scenarios/drive-escalation-wash-priority.ron` (modify, almost certainly to add an override)
- `crates/worldwake-cli/tests/scenario_lint_sweep.rs` (new — CI iterator test)
- `crates/worldwake-cli/src/lib.rs` (modify — only if `scenario` module is not already publicly re-exported; verify during implementation)

## Out of Scope

- The lint rules themselves (S111SCEHOMLIN-002).
- The override mechanism plumbing (S111SCEHOMLIN-003).
- The PlanningSnapshot doctest (S111SCEHOMLIN-001).
- Adding new lint rules.
- Restructuring scenario file format beyond adding `scenario_lint_overrides` and (for fix-path scenarios) varying existing profile fields.
- Touching scenarios outside `scenarios/` (e.g., inline RON in test files).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli --test scenario_lint_sweep` (the new CI iterator passes — every committed scenario passes lints or has valid overrides).
2. `cargo test -p worldwake-cli` (no regression in existing CLI tests, including `tests/integration.rs` and `tests/golden_observer_anomalies.rs`).
3. Existing AI golden tests on the affected scenarios continue to pass: `cargo test -p worldwake-ai golden_survival` and `cargo test -p worldwake-ai golden_drive_escalation_wash_priority`. Behavioral fixes (varying profile fields) must not break the golden invariants those tests assert; if a fix would break a golden, the override path is the correct choice for that scenario.
4. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Every file in `scenarios/` either passes `run_lints` with zero failures, or has every failure covered by a `scenario_lint_overrides` entry with a non-empty, content-bearing justification.
2. The CI sweep test discovers scenarios via filesystem read, so adding a new `.ron` file under `scenarios/` automatically subjects it to the lint contract.
3. Scenarios fixed by varying profile fields preserve the existing golden-test contracts they participate in (no behavioral regression).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/tests/scenario_lint_sweep.rs` (new) — iterates `scenarios/*.ron` and asserts every scenario passes lints. Rationale: turns the load-time contract into a CI-enforced repository invariant per FND-31.
2. Each modified scenario file is exercised by its existing golden test (named in Acceptance Criteria #3); no new per-scenario tests are added.

### Commands

1. `cargo test -p worldwake-cli --test scenario_lint_sweep` (targeted — new CI test)
2. `cargo test -p worldwake-cli` (CLI crate regression including integration + golden_observer_anomalies)
3. `cargo test -p worldwake-ai golden_survival golden_drive_escalation_wash_priority` (verify scenario behavioral fixes don't break existing goldens)
4. `cargo clippy --workspace --all-targets -- -D warnings` (CI parity)
