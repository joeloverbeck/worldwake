# S152COGARCSEE-007: Scenario diagnostics archetype distribution

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — diagnostics report field (`worldwake-ai`)
**Deps**: archive/tickets/S152COGARCSEE-002.md, archive/tickets/S152COGARCSEE-005.md

## Problem

Before this ticket, S144 scenario diagnostics did not surface the archetype distribution, so a scenario's diversity was not auditable through the diagnostics report (FND-29). This ticket added `agent_archetypes: BTreeMap<CognitiveArchetype, u64>` to `ScenarioDiagnosticsReport`, counted across agents.

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ScenarioDiagnosticsReport` is at `crates/worldwake-ai/src/scenario_diagnostics/mod.rs`, deriving `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`; adding a `BTreeMap` field is derive-safe. It is built by `build_scenario_diagnostics` (`mod.rs`); the aggregation lives in `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`.
2. `CognitiveArchetype` (ticket 001) derives `Ord` (a valid `BTreeMap` key); `CognitiveArchetypeComponent` (ticket 002) is read from authoritative world state, populated by ticket 005.
3. Boundary under audit: the diagnostics aggregation reads the `CognitiveArchetypeComponent` getter across agents and tallies counts. Determinism uses `BTreeMap` (AGENTS.md invariant).
4. (Mismatch + correction) Counting reads the agent's own authoritative component — no belief view is needed (the diagnostics report is a tooling/aggregation surface, not an agent decision). Live reassessment also found that `build_scenario_diagnostics` previously had no `World` input, so this ticket reshaped that report builder signature and updated existing callers to pass the live world.

## Architecture Check

1. A derived count over authoritative component state (FND-27 cache/summary, never truth) — recomputable from world state, stored only in the report.
2. No backwards-compatibility concern: additive field on a report struct.

## Verified Layers

1. The report tallies archetypes correctly -> scenario diagnostics fixture coverage compares `ScenarioDiagnosticsReport.agent_archetypes` against a direct live-world component count (derived read-model surface).
2. Single-layer ticket (diagnostics aggregation); no decision/action-trace layer applies because it reads authoritative state directly.

## Landed Changes

### 1. Report field added

Added `pub agent_archetypes: BTreeMap<CognitiveArchetype, u64>` to `ScenarioDiagnosticsReport` (`scenario_diagnostics/mod.rs`).

### 2. Aggregator populated from world state

`build_scenario_diagnostics` now accepts `&World`, iterates live agents, reads `CognitiveArchetypeComponent.archetype`, and tallies into the map.

### 3. Public diagnostics surfaces updated

The observer diagnostics text renders an "Agent archetype distribution" table. The observer JSON adapter serializes the map as stable key/count entries and accepts omitted `agent_archetypes` on old JSON payloads via a serde default.

## Landed Files

- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (modified — report field and serde roundtrip fixture)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modified — `&World` report input and tally)
- `crates/worldwake-ai/tests/scenario_diagnostics_harness/mod.rs` (modified — live-world distribution assertion and builder call)
- `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (regenerated — public fixture includes archetype distribution)
- `crates/worldwake-cli/src/diagnostics_json.rs` (modified — JSON adapter support)
- `crates/worldwake-cli/src/bin/observer.rs` (modified — text rendering and focused observer tests)

## Out of Scope

- Per-agent observer labeling (ticket 006 covered the per-agent label; this ticket only added the aggregate diagnostics distribution).
- Any engine/state mutation.

## Acceptance Result

### Behavior Proved

1. A spawned scenario diagnostics run now compares `ScenarioDiagnosticsReport.agent_archetypes` against a direct count over `CognitiveArchetypeComponent` values in the live `World`.
2. Existing focused diagnostics coverage passes with the updated report shape: `cargo test -p worldwake-ai scenario_diagnostics`.

### Invariants Proved

1. `agent_archetypes` is a derived count over authoritative component state, never a source of truth (FND-27).
2. Map iteration is deterministic (`BTreeMap`).

## Test Plan Result

### Modified Tests

1. `crates/worldwake-ai/src/scenario_diagnostics/` — report serde tests now include `agent_archetypes`; aggregator unit tests pass through an explicit empty `World` where archetype counts are irrelevant.
2. `crates/worldwake-ai/tests/scenario_diagnostics_harness/mod.rs` — scenario diagnostics fixture path asserts the report distribution equals a direct live-world archetype count.
3. `crates/worldwake-cli/src/bin/observer.rs` — focused observer tests assert text and JSON diagnostics surface the new field.

### Commands Run

1. `cargo test -p worldwake-ai scenario_diagnostics`
2. `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact`
3. `cargo test -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact`
4. `cargo test -p worldwake-cli --bin observer render_scenario_diagnostics_section`
5. `cargo test -p worldwake-ai`
6. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-20.

- Added the `ScenarioDiagnosticsReport.agent_archetypes` derived read-model and populated it from live authoritative `CognitiveArchetypeComponent` state.
- Threaded the read-only `&World` diagnostics input through existing diagnostics callers.
- Surfaced the distribution in observer text and JSON diagnostics.
- Regenerated the scenario diagnostics JSON fixture through the existing ignored fixture update path.

## Deviations

- The drafted aggregator API had no live `World` parameter to read authoritative components from. The landed implementation reshaped `build_scenario_diagnostics` to accept `&World` rather than deriving the distribution from traces or event-log payloads.
- The landed surface included observer JSON/text adapter and fixture fallout because `ScenarioDiagnosticsReport` is a public diagnostics payload, not only an internal `worldwake-ai` struct.

## Verification Result

- Passed `cargo test -p worldwake-ai scenario_diagnostics`.
- Passed `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable -- --ignored --exact`.
- Passed `cargo test -p worldwake-cli --bin observer render_scenario_diagnostics_section`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
