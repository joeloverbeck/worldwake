# S117CONMAIOBS-007: Golden coverage and scenario fixtures

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Observer-only
**Deps**: `archive/tickets/S117CONMAIOBS-001.md`, `archive/tickets/S117CONMAIOBS-002.md`, `archive/tickets/S117CONMAIOBS-003.md`, `archive/tickets/S117CONMAIOBS-004.md`, `archive/tickets/S117CONMAIOBS-005.md`, `archive/tickets/S117CONMAIOBS-006.md`, `archive/tickets/S117CONMAIOBS-009.md`, `archive/tickets/S117CONMAIOBS-010.md`, `S117CONMAIOBS-013`, `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

Four new observer detectors (002–005) and two supplementary tables (006) ship without end-to-end regression coverage. Without goldens, the detectors can silently drift — a threshold tuning, a rename, or a refactor could turn a real anomaly into no-op or a baseline run into a false-positive flood, and the `/scenario-analysis` skill's downstream consumers would inherit the regression. This ticket writes four scripted-scenario goldens plus fixture `.ron` files that force each detector's trigger path and assert both positive detection and absence of false positives where appropriate.

## Assumption Reassessment (2026-04-18)

1. The observer binary's scenario load path uses `load_scenario_file()` + `spawn_scenario()` (see `crates/worldwake-cli/src/scenario/mod.rs`), but there is no existing public observer report helper to call from an integration test. The honest E2E seam is the compiled `observer` binary itself, invoked from the test via `env!("CARGO_BIN_EXE_observer")` with a temp output path. Existing `crates/worldwake-cli/tests/integration.rs` does not already cover the observer report pipeline; `test_observer_mode_simulation_runs` only proves observer-mode ticking through the interactive CLI stack.
2. `MetabolismProfile.wilderness_relief_dirtiness_penalty` is scenario-configurable (`crates/worldwake-core/src/needs.rs:149` and `crates/worldwake-cli/src/scenario/types.rs`). Confirmed during the S117 reassessment.
3. Scenario fixtures for goldens live in a new directory `crates/worldwake-cli/tests/fixtures/observer_anomalies/` — the existing production scenarios under `scenarios/` are reserved for the real simulation; test fixtures are committed alongside the tests that use them.
4. Shared abstraction boundary under audit: the observer's report-generation pipeline — specifically the `Vec<Anomaly>` produced by `detect_anomalies()` and the Section 3 render path. Goldens assert string-level content matching in the generated report, not byte-identical output (so minor whitespace changes don't create spurious test failures).
5. Scenario isolation: each fixture is intentionally minimal. The `convergence` fixture proves only that `GEOGRAPHIC_CONVERGENCE` fires; the `maintenance_starvation` fixture proves only dirtiness starvation; etc. Cross-detector interactions (e.g., a scenario that triggers both convergence and starvation) are out of scope for this ticket.
6. Lawful competing affordances intentionally excluded from each fixture:
   - Convergence fixture: only one viable place per need; agents cannot rotate.
   - Maintenance starvation fixture: wash facility is far enough that relief cadence lags accumulation; no alternative washing mechanism.
   - Recipe monoculture fixture: both food recipes are known, but only Agent A receives final `FieldPlot` workstation evidence in its belief store; Agent B never perceives that workstation. This matches the landed live gate, which checks final workstation evidence rather than a stronger source-viability proof.
   - Acute need spike fixture: the scenario forces one bounded thirst-critical run in the detector's 30–99 tick window, then relief; exact start/end ticks are asserted from the live report output rather than hardcoded from the draft narrative.

## Architecture Check

1. Using fixture `.ron` files committed alongside tests is the correct separation: production scenarios stay in `scenarios/`, test fixtures stay under `tests/fixtures/`. No shared-authority state exists between them.
2. Goldens assert on generated report strings using substring matching plus numeric bounds (e.g., anomaly count, specific tick ranges), not byte-exact matching — this keeps the goldens robust against minor format cosmetics while pinning the semantic contract.
3. No backward-compatibility shim — goldens are greenfield tests against the landed detector behavior.

## Verification Layers

1. Detector fires on its forcing fixture → golden E2E assertion on Section 3 content.
2. Detector does NOT fire on `scenarios/survival-baseline.ron` (healthy baseline) → regression guard command in Test Plan; this test asserts the anomaly count for each new variant is zero on the baseline run.
3. Belief-gate negative case for `RecipeMonoculture` → asserted in the same test that covers the positive case (control-group agent lacks grainfield belief and does NOT trigger).
4. Mixed-layer ticket: (a) detector emission correctness → golden E2E via report-string assertion; (b) Section 3 rendering correctness → same assertion surface, checked via string contents; (c) report format stability → existing `test_observer_mode_simulation_runs` is the backstop (continues to pass).

## What to Change

### 1. Scenario fixtures

Create four `.ron` files under `crates/worldwake-cli/tests/fixtures/observer_anomalies/`:

- `convergence_hub.ron` — 3 agents, 2 places (HubPlace and AuxPlace). Place design + agent profiles ensure all 3 agents spend ≥60% of ticks at HubPlace over a 300-tick run. Needs are tuned to be satisfiable at HubPlace only.
- `maintenance_starvation_wash_gap.ron` — 2 agents, 3 places (Home, Wash, Forest). `wilderness_relief_dirtiness_penalty = 200`. Wash facility is 5 travel hops away. Hunger/thirst/bladder satisfiable near Home; dirtiness only at Wash. Over 600 ticks, dirtiness accumulates faster than the agents can travel to Wash and back.
- `recipe_monoculture_apples_vs_grain.ron` — 2 agents with identical `KnownRecipes = {Harvest Apples, Harvest Grain}`. Agent A starts co-located with both an orchard and a `FieldPlot` workstation, while Agent B only ever sees an orchard. Apple harvesting remains the only repeatedly used hunger action for both agents; Agent A should trigger the anomaly because the final belief store still carries `FieldPlot` evidence, Agent B should not.
- `acute_thirst_spike.ron` — 1 agent with thirst/metabolism/topology tuned so one thirst-critical run lands inside the detector's 30–99 tick acute window and then resolves. The live report's tick range becomes the asserted contract.

Each fixture uses the existing scenario schema — no new fields. Cross-reference structural conventions with `scenarios/survival-baseline.ron` and `scenarios/survival-contested.ron`.

### 2. Golden test file

Create `crates/worldwake-cli/tests/golden_observer_anomalies.rs` with four tests that invoke `env!("CARGO_BIN_EXE_observer")`, read the generated markdown report from a temp file, and assert on Section 3 content:

```rust
#[test]
fn convergence_smell_fires_on_forced_hub_scenario() {
    let report = run_observer("tests/fixtures/observer_anomalies/convergence_hub.ron", 300);
    // Exactly one GEOGRAPHIC_CONVERGENCE anomaly; all 3 agents named in header.
    assert_eq!(count_anomalies_of_kind(&report, "GEOGRAPHIC_CONVERGENCE"), 1);
    assert!(report.contains("GEOGRAPHIC_CONVERGENCE"));
    // Header includes all three agent names (order: lead + additional sorted by EntityId).
    // ... specific agent-name substring checks.
}

#[test]
fn maintenance_starvation_fires_on_wash_gap() {
    let report = run_observer("tests/fixtures/observer_anomalies/maintenance_starvation_wash_gap.ron", 600);
    // MAINTENANCE_STARVATION for Dirtiness on each of the 2 agents.
    assert_eq!(count_anomalies_of_kind(&report, "MAINTENANCE_STARVATION"), 2);
    // Description contains "Dirtiness" and both "accumulated" and "relieved" with accumulation > relief.
}

#[test]
fn recipe_monoculture_fires_on_single_food_dependency() {
    let report = run_observer("tests/fixtures/observer_anomalies/recipe_monoculture_apples_vs_grain.ron", 1000);
    // Exactly one RECIPE_MONOCULTURE anomaly (Agent A with grainfield belief), not two.
    assert_eq!(count_anomalies_of_kind(&report, "RECIPE_MONOCULTURE"), 1);
    // Description mentions Harvest Apples and Harvest Grain.
    // Agent B (no grainfield belief) does not appear in any RECIPE_MONOCULTURE anomaly header.
}

#[test]
fn acute_need_spike_fires_on_40_tick_thirst() {
    let report = run_observer("tests/fixtures/observer_anomalies/acute_thirst_spike.ron", 200);
    assert_eq!(count_anomalies_of_kind(&report, "ACUTE_NEED_SPIKE"), 1);
    // The bounded acute run is rendered in the live report output.
    // Tick range in description is 1–40 or equivalent.
}
```

Test helpers `run_observer(scenario_path, ticks) -> String`, `count_anomalies_of_kind(report: &str, kind: &str) -> usize`, and small anomaly-section extractors live in the same test file.

### 3. Baseline regression command

The `Test Plan` below names an explicit command that runs the observer against `scenarios/survival-baseline.ron` and asserts zero anomalies of the four new variants. This runs on demand rather than in the test suite (it takes ~minutes) — the developer runs it before merging.

## Files to Touch

- `crates/worldwake-cli/tests/fixtures/observer_anomalies/convergence_hub.ron` (new)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/maintenance_starvation_wash_gap.ron` (new)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/recipe_monoculture_apples_vs_grain.ron` (new)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/acute_thirst_spike.ron` (new)
- `crates/worldwake-cli/tests/golden_observer_anomalies.rs` (new)
- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Cross-detector scenarios (e.g., a fixture that triggers both convergence and starvation simultaneously).
- Tuning fixture numerical values to match specific real-world patterns — the fixtures exist to force detector trigger paths, not to recreate survival-contested.
- Adding the baseline regression assertion to the automated test suite — it is a command, not a `#[test]`.

## Acceptance Criteria

### Tests That Must Pass

1. `convergence_smell_fires_on_forced_hub_scenario` passes.
2. `maintenance_starvation_fires_on_wash_gap` passes.
3. `recipe_monoculture_fires_on_single_food_dependency` passes.
4. `acute_need_spike_fires_on_bounded_thirst_run` passes.
5. `cargo test -p worldwake-cli --test golden_observer_anomalies` runs all four.
6. Existing integration: `test_observer_mode_simulation_runs` still passes.
7. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. Each fixture's agent count, place graph, and profile configuration exactly matches the scenario schema fields used by `load_scenario_file` (no deprecated fields, no invented fields).
2. Each positive-case test asserts exact count for the target anomaly kind, not `>= 1` — drift detection.
3. The `recipe_monoculture` fixture's belief-gate control case (Agent B without grainfield belief) is asserted: Agent B's name must NOT appear in any `RECIPE_MONOCULTURE` anomaly header.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/tests/golden_observer_anomalies.rs` — four end-to-end goldens above.
2. `crates/worldwake-cli/tests/fixtures/observer_anomalies/*.ron` — four scenario fixtures.

### Commands

1. `cargo test -p worldwake-cli --test golden_observer_anomalies`
2. `cargo test -p worldwake-cli` (full crate suite, including existing integration tests)
3. Baseline regression (manual; not part of automated suite): `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md && grep -c "GEOGRAPHIC_CONVERGENCE\|MAINTENANCE_STARVATION\|RECIPE_MONOCULTURE\|ACUTE_NEED_SPIKE" /tmp/baseline-dump.md` — expected output: `0`. Verify the observer CLI flag names (`--ticks`, `--output`) against `bin/observer.rs` argument parsing before running; if they differ, use the actual names.
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-18.

- Added `crates/worldwake-cli/tests/golden_observer_anomalies.rs`, which drives the compiled `observer` binary through `env!("CARGO_BIN_EXE_observer")`, reads the emitted markdown report, and asserts exact anomaly-count contracts plus key header/description content for convergence, maintenance starvation, recipe monoculture, and acute need spike.
- Added four scenario fixtures under `crates/worldwake-cli/tests/fixtures/observer_anomalies/`:
  - `convergence_hub.ron`
  - `maintenance_starvation_wash_gap.ron`
  - `recipe_monoculture_apples_vs_grain.ron`
  - `acute_thirst_spike.ron`
- Fixed an observer-side production mismatch exposed by the new goldens: recipe usage and recipe-monoculture counting now normalize real action-trace names like `harvest:Harvest Apples` / `craft:Bake Bread` back to canonical registry recipe names before counting commits. That same fix makes Section 2 `Recipe usage` rows honest on live runs instead of silently rendering `0` for committed known recipes.

## Deviations

- The drafted ticket assumed an existing public observer report helper. Live reassessment showed no such seam exists, so the landed goldens use the compiled `observer` binary itself via `env!("CARGO_BIN_EXE_observer")` and a temp output file.
- The drafted ticket claimed `Engine Changes: None`, but the golden pass exposed a real observer read-side contradiction in production code: `recipe_usage_rows()` and `detect_recipe_monoculture()` were counting canonical recipe names while live action traces record prefixed action names (`harvest:` / `craft:`). This ticket absorbed the narrow observer-side fix rather than shipping a knowingly broken golden.
- The drafted acute fixture narrative hardcoded a 40-tick remote-water path. Live scenario-authoring constraints made that exact branch brittle, so the landed fixture proves the same detector contract through a local well plus sleep-first pressure competition. The test asserts the live bounded acute anomaly rather than preserving the stale narrative tick math verbatim.
- The draft manual baseline regression command's `grep -c "GEOGRAPHIC_CONVERGENCE\|MAINTENANCE_STARVATION\|RECIPE_MONOCULTURE\|ACUTE_NEED_SPIKE"` check is too loose for a markdown report because it counts any mention of the labels, not just anomaly headers. Post-ticket review reran the baseline proof with a header-level check and found real false positives in `scenarios/survival-baseline.ron`, so this ticket remains active until the remaining maintenance-starvation and acute-spike regressions are dispositioned honestly.

## Verification Result

- Passed `cargo test -p worldwake-cli --test golden_observer_anomalies`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed automated convergence baseline regression in `crates/worldwake-cli/tests/golden_observer_anomalies.rs`: `GEOGRAPHIC_CONVERGENCE` stays absent on `scenarios/survival-baseline.ron`
- Remaining baseline regression is still failing outside convergence: the healthy baseline run emits corrected `MAINTENANCE_STARVATION` windows and `ACUTE_NEED_SPIKE` anomaly headers. The maintenance-only disposition completed in archived `S117CONMAIOBS-012.md`, and the acute/split-support disposition completed in `archive/tickets/S117CONMAIOBS-011.md`; the remaining implementation blocker is now `S117CONMAIOBS-013`
