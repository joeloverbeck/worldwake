# S152COGARCSEE-007: Scenario diagnostics archetype distribution

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — diagnostics report field (`worldwake-ai`)
**Deps**: S152COGARCSEE-002, S152COGARCSEE-005

## Problem

S144 scenario diagnostics should surface the archetype distribution so a scenario's diversity is auditable (FND-29). S152 adds `agent_archetypes: BTreeMap<CognitiveArchetype, u64>` to `ScenarioDiagnosticsReport`, counted across agents.

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ScenarioDiagnosticsReport` is at `crates/worldwake-ai/src/scenario_diagnostics/mod.rs:14`, deriving `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`; adding a `BTreeMap` field is derive-safe. It is built by `build_scenario_diagnostics` (`mod.rs:11`); the aggregation lives in `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`.
2. `CognitiveArchetype` (ticket 001) derives `Ord` (a valid `BTreeMap` key); `CognitiveArchetypeComponent` (ticket 002) is read from authoritative world state, populated by ticket 005.
3. Boundary under audit: the diagnostics aggregation reads the `CognitiveArchetypeComponent` getter across agents and tallies counts. Determinism uses `BTreeMap` (CLAUDE.md invariant).
4. (Mismatch + correction) Counting reads the agent's own authoritative component — no belief view is needed (the diagnostics report is a tooling/aggregation surface, not an agent decision).

## Architecture Check

1. A derived count over authoritative component state (FND-27 cache/summary, never truth) — recomputable from world state, stored only in the report.
2. No backwards-compatibility concern: additive field on a report struct.

## Verification Layers

1. The report tallies archetypes correctly -> focused unit test on `build_scenario_diagnostics`/aggregator with a known agent population (derived read-model surface).
2. Single-layer ticket (diagnostics aggregation); no decision/action-trace layer applies because it reads authoritative state directly.

## What to Change

### 1. Add the report field

Add `pub agent_archetypes: BTreeMap<CognitiveArchetype, u64>` to `ScenarioDiagnosticsReport` (`scenario_diagnostics/mod.rs`).

### 2. Populate in the aggregator

In `build_scenario_diagnostics`/`aggregator.rs`, iterate agents, read `CognitiveArchetypeComponent.archetype`, and tally into the map.

## Files to Touch

- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (modify — field)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — tally)

## Out of Scope

- Observer rendering of the distribution (ticket 006 covers per-agent label; report consumers are separate).
- Any engine/state mutation.

## Acceptance Criteria

### Tests That Must Pass

1. A scenario with a known archetype mix produces the expected `agent_archetypes` counts (sum equals agent count).
2. Existing suite: `cargo test -p worldwake-ai scenario_diagnostics`

### Invariants

1. `agent_archetypes` is a derived count over authoritative component state, never a source of truth (FND-27).
2. Map iteration is deterministic (`BTreeMap`).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/scenario_diagnostics/` (`#[cfg(test)]` in `mod.rs` or `aggregator.rs`) — archetype tally over a known population.

### Commands

1. `cargo test -p worldwake-ai scenario_diagnostics`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `./scripts/verify.sh`
