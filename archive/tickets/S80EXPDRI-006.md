# S80EXPDRI-006: Keep exploration counters runtime-only in scenario authoring

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — scenario authoring shape must stop exposing runtime-only exploration counter state
**Deps**: S80EXPDRI-002

## Problem

`S80EXPDRI-002` made `ExplorationProfile` scenario-definable by adding `AgentDef.exploration_profile: Option<ExplorationProfile>`, but that also exposed `consecutive_exploration_count`. The active spec says `consecutive_exploration_count` is runtime-only and must always start at `0`; today scenario RON and `spawn_agent()` can seed a nonzero value, which collapses a runtime progression field into authored bootstrap state.

## Assumption Reassessment (2026-04-10)

1. `ExplorationProfile` in `crates/worldwake-core/src/exploration.rs` currently contains both disposition fields (`curiosity_weight`, `need_activation_threshold`, `max_consecutive_explorations`, `visit_lookback_ticks`) and the runtime progression field `consecutive_exploration_count`.
2. `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs` currently exposes `pub exploration_profile: Option<ExplorationProfile>`, so scenario deserialization accepts `consecutive_exploration_count` directly.
3. `spawn_agent()` in `crates/worldwake-cli/src/scenario/mod.rs` currently applies `agent_def.exploration_profile.unwrap_or_default()` directly via `set_component_exploration_profile`, so bootstrap preserves any authored nonzero counter.
4. The current CLI tests prove this leak explicitly: `test_scenario_def_deserialize_full` in `crates/worldwake-cli/src/scenario/types.rs` deserializes `consecutive_exploration_count: 1`, and `test_spawn_agent_with_profile_overrides` in `crates/worldwake-cli/src/scenario/mod.rs` asserts that the spawned world keeps that nonzero value.
5. The active spec in `specs/S80-exploration-drive.md` says `consecutive_exploration_count` is "Not scenario-definable — always starts at 0" and that `AgentDef` should expose all `ExplorationProfile` fields except that runtime-only counter.
6. Shared abstraction boundary under audit: CLI scenario authoring (`AgentDef` / RON schema) versus authoritative runtime state (`ExplorationProfile` component on spawned agents). The clean contract is authored disposition in CLI, runtime counter in authoritative state only.

## Architecture Check

1. The clean fix is to separate authored exploration disposition from runtime-owned counter state at the scenario boundary, rather than treating the full authoritative component as a RON schema. That preserves the spec's stated contract and keeps authored bootstrap data from seeding in-flight runtime progression.
2. No backward-compatibility shim is needed. The scenario-facing shape can be corrected directly because the current exposure was introduced by the just-landed ticket and has not yet become a stable compatibility contract.

## Verification Layers

1. Scenario RON cannot author `consecutive_exploration_count` -> focused scenario deserialization test
2. `spawn_agent()` always seeds `consecutive_exploration_count` to `0` while preserving other exploration disposition fields -> focused scenario spawn test
3. Single-boundary ticket (CLI authoring/bootstrap to authoritative component); broader layer mapping not applicable

## What to Change

### 1. Split scenario-facing exploration authoring from runtime component shape

In `crates/worldwake-cli/src/scenario/types.rs`, replace direct `Option<ExplorationProfile>` authoring with a scenario-facing shape that includes only:
- `curiosity_weight`
- `need_activation_threshold`
- `max_consecutive_explorations`
- `visit_lookback_ticks`

Do not allow RON input to provide `consecutive_exploration_count`.

### 2. Seed the authoritative component with runtime-safe defaults

In `crates/worldwake-cli/src/scenario/mod.rs`, when `spawn_agent()` writes `ExplorationProfile`:
- preserve authored disposition values
- force `consecutive_exploration_count` to `0`

### 3. Rewrite focused tests to prove the corrected boundary

- Update scenario deserialization coverage so `exploration_profile` round-trips only the authored fields
- Replace the current nonzero-counter spawn assertion with proof that spawned agents start at `0`

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — introduce scenario-facing exploration authoring shape and tests)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — convert authored exploration config into authoritative `ExplorationProfile` with runtime counter reset)

## Out of Scope

- Candidate generation, ranking, or planner integration for `ExploreLocation`
- Agent-tick counter increment/reset behavior (owned by `S80EXPDRI-004`)
- Changes to the authoritative `ExplorationProfile` component shape in `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. Scenario deserialization accepts authored exploration disposition fields without exposing `consecutive_exploration_count`
2. Spawned agents always start with `ExplorationProfile.consecutive_exploration_count == 0`
3. Authored exploration disposition values still survive bootstrap unchanged
4. Existing suite: `cargo test -p worldwake-cli`
5. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `consecutive_exploration_count` remains runtime-owned state, not scenario-authored bootstrap data
2. `ExplorationProfile` remains scenario-definable for the disposition fields required by the spec
3. No new compatibility alias or duplicate bootstrap path is introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/types.rs` — deserialization coverage for the scenario-facing exploration shape
2. `crates/worldwake-cli/src/scenario/mod.rs` — bootstrap coverage proving runtime counter resets to `0`

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo build --workspace`

## Outcome

Completed on 2026-04-10.

- Replaced direct scenario authoring of `ExplorationProfile` with a dedicated `ExplorationProfileDef` in `crates/worldwake-cli/src/scenario/types.rs` so RON can author only the disposition fields and cannot provide the runtime-only `consecutive_exploration_count`.
- Added `#[serde(deny_unknown_fields)]` coverage for the scenario-facing exploration shape and a focused test proving `consecutive_exploration_count` is rejected during deserialization.
- Updated `spawn_agent()` in `crates/worldwake-cli/src/scenario/mod.rs` to convert the authored disposition into an authoritative `worldwake_core::ExplorationProfile` while always forcing `consecutive_exploration_count` to `0`.
- Updated the focused CLI bootstrap tests so authored exploration values survive spawn unchanged except for the runtime-owned counter reset.

## Verification Result

- Passed `cargo test -p worldwake-cli test_exploration_profile_def_rejects_runtime_counter_field`
- Passed `cargo test -p worldwake-cli test_spawn_agents_receive_default_universal_profiles`
- Passed `cargo test -p worldwake-cli test_spawn_agent_with_profile_overrides`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo build --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
