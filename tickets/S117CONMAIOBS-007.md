# S117CONMAIOBS-007: Golden coverage and scenario fixtures

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: `archive/tickets/S117CONMAIOBS-001.md`, `archive/tickets/S117CONMAIOBS-002.md`, `archive/tickets/S117CONMAIOBS-003.md`, `archive/tickets/S117CONMAIOBS-004.md`, `archive/tickets/S117CONMAIOBS-005.md`, `S117CONMAIOBS-006`, `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

Four new observer detectors (002–005) and two supplementary tables (006) ship without end-to-end regression coverage. Without goldens, the detectors can silently drift — a threshold tuning, a rename, or a refactor could turn a real anomaly into no-op or a baseline run into a false-positive flood, and the `/scenario-analysis` skill's downstream consumers would inherit the regression. This ticket writes four scripted-scenario goldens plus fixture `.ron` files that force each detector's trigger path and assert both positive detection and absence of false positives where appropriate.

## Assumption Reassessment (2026-04-18)

1. The observer binary's scenario load path uses `load_scenario_file()` + `spawn_scenario()` (see `crates/worldwake-cli/src/scenario/mod.rs`). Goldens invoke the observer's report-generation pipeline directly rather than shelling out to the binary; this is consistent with existing `crates/worldwake-cli/tests/integration.rs` patterns (see `test_observer_mode_simulation_runs` at line 395).
2. `MetabolismProfile.wilderness_relief_dirtiness_penalty` is scenario-configurable (`crates/worldwake-core/src/needs.rs:149` and `crates/worldwake-cli/src/scenario/types.rs`). Confirmed during the S117 reassessment.
3. Scenario fixtures for goldens live in a new directory `crates/worldwake-cli/tests/fixtures/observer_anomalies/` — the existing production scenarios under `scenarios/` are reserved for the real simulation; test fixtures are committed alongside the tests that use them.
4. Shared abstraction boundary under audit: the observer's report-generation pipeline — specifically the `Vec<Anomaly>` produced by `detect_anomalies()` and the Section 3 render path. Goldens assert string-level content matching in the generated report, not byte-identical output (so minor whitespace changes don't create spurious test failures).
5. Scenario isolation: each fixture is intentionally minimal. The `convergence` fixture proves only that `GEOGRAPHIC_CONVERGENCE` fires; the `maintenance_starvation` fixture proves only dirtiness starvation; etc. Cross-detector interactions (e.g., a scenario that triggers both convergence and starvation) are out of scope for this ticket.
6. Lawful competing affordances intentionally excluded from each fixture:
   - Convergence fixture: only one viable place per need; agents cannot rotate.
   - Maintenance starvation fixture: wash facility is far enough that relief cadence lags accumulation; no alternative washing mechanism.
   - Recipe monoculture fixture: both food recipes are known but the grainfield facility is intentionally out of the agent's perception-reachable radius (belief-gate control group) for the negative case.
   - Acute need spike fixture: the scenario forces a 40-tick thirst run followed by relief so the run is strictly bounded at 40 ticks.

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
- `recipe_monoculture_apples_vs_grain.ron` — 2 agents with identical `KnownRecipes = {Harvest Apples, Harvest Grain}`. Agent A sees the grainfield facility during startup (belief seeded). Agent B does not (no perception reach). Apple orchard is co-located with both; grainfield is at a distant place. Over 1000 ticks, both agents eat apples exclusively; Agent A should trigger the anomaly, Agent B should not.
- `acute_thirst_spike.ron` — 1 agent with `dehydration_tolerance_ticks = 220` and metabolism tuned so thirst rises from 0 to 900 over 40 ticks (peak at tick 40), then a water source becomes available at tick 41 and thirst drops to 100 by tick 50. Over 200 ticks total.

Each fixture uses the existing scenario schema — no new fields. Cross-reference structural conventions with `scenarios/survival-baseline.ron` and `scenarios/survival-contested.ron`.

### 2. Golden test file

Create `crates/worldwake-cli/tests/golden_observer_anomalies.rs` with four tests:

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
    // No overlap with SUSTAINED_CRITICAL_NEED for the same agent.
    // Tick range in description is 1–40 or equivalent.
}
```

Test helpers `run_observer(scenario_path, ticks) -> String` and `count_anomalies_of_kind(report: &str, kind: &str) -> usize` live in the same test file or a shared `tests/common/` module.

### 3. Baseline regression command

The `Test Plan` below names an explicit command that runs the observer against `scenarios/survival-baseline.ron` and asserts zero anomalies of the four new variants. This runs on demand rather than in the test suite (it takes ~minutes) — the developer runs it before merging.

## Files to Touch

- `crates/worldwake-cli/tests/fixtures/observer_anomalies/convergence_hub.ron` (new)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/maintenance_starvation_wash_gap.ron` (new)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/recipe_monoculture_apples_vs_grain.ron` (new)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/acute_thirst_spike.ron` (new)
- `crates/worldwake-cli/tests/golden_observer_anomalies.rs` (new)

## Out of Scope

- Cross-detector scenarios (e.g., a fixture that triggers both convergence and starvation simultaneously).
- Tuning fixture numerical values to match specific real-world patterns — the fixtures exist to force detector trigger paths, not to recreate survival-contested.
- Adding the baseline regression assertion to the automated test suite — it is a command, not a `#[test]`.

## Acceptance Criteria

### Tests That Must Pass

1. `convergence_smell_fires_on_forced_hub_scenario` passes.
2. `maintenance_starvation_fires_on_wash_gap` passes.
3. `recipe_monoculture_fires_on_single_food_dependency` passes.
4. `acute_need_spike_fires_on_40_tick_thirst` passes.
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
