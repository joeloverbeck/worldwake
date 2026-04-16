# S105OBSSALFIL-001: Add `observation_budget` field to `PerceptionProfile`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `PerceptionProfile` struct in worldwake-core
**Deps**: None

## Problem

The perception pipeline has no per-agent attention limit on how many co-located entities are observed per tick. Adding the `observation_budget` field to `PerceptionProfile` is the prerequisite for the priority-and-truncate pipeline in S105OBSSALFIL-002. This ticket adds the field, its default, serde compatibility, and updates the explicit full-struct construction sites that must remain exhaustive after the shared-field addition.

## Assumption Reassessment (2026-04-16)

1. `PerceptionProfile` in `crates/worldwake-core/src/belief.rs` already derives `Copy`, `Clone`, `Serialize`, and `Deserialize`. Adding a `u8` field is mechanically safe if the field has an explicit serde default for authored scenario input.
2. The draft overstated the constructor fallout. Many `PerceptionProfile` call sites in `worldwake-sim`, `worldwake-systems`, and `worldwake-ai` already use `..PerceptionProfile::default()` and required no edit. The honest edit surface is the struct definition, the `Default` impl, the local core test helper, and the remaining exhaustive `PerceptionProfile { ... }` literals that compile-failed after the field landed.
3. Existing `.ron` scenarios still define `perception_profile` without `observation_budget`, so the new field must deserialize with `#[serde(default = "default_observation_budget")]`.
4. `AgentDef` still carries `perception_profile: Option<PerceptionProfile>`, and `spawn_agent()` still uses `unwrap_or_default()`. Agents that omit the profile entirely therefore continue to receive the universal default profile automatically.
5. The honest focused proof surface is not “build only.” `worldwake-core` already had a default-profile test, and `worldwake-cli` already had scenario/per-profile tests that could be extended to prove explicit authored input and omitted-field defaulting.

## Architecture Check

1. Adding a direct `u8` field with a serde default is the minimal, non-shim change that keeps existing authored scenarios loading while making the budget available to later S105 pipeline work.
2. No behavior changes landed in this ticket. The field is stored and defaulted only; observation ordering and truncation remain owned by S105OBSSALFIL-002.

## Verification Layers

1. Shared-field constructor fallout compiles across all targets.
2. `PerceptionProfile::default()` and core serde behavior prove the field defaults to `24`.
3. Scenario/authored-input parsing in `worldwake-cli` proves explicit `observation_budget` input and omitted-field defaulting.
4. Full workspace regression and CI-matching clippy remain green.

## What to Change

### 1. Add `observation_budget` to `PerceptionProfile`

In `crates/worldwake-core/src/belief.rs`:
- add `pub observation_budget: u8` after `observation_buffer_capacity`
- add `#[serde(default = "default_observation_budget")]`
- add `fn default_observation_budget() -> u8 { 24 }`
- update `Default` to set `observation_budget: 24`

### 2. Update exhaustive `PerceptionProfile` literals

Add `observation_budget` to the remaining full literals in:
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/belief.rs`
- `crates/worldwake-systems/src/perception.rs`
- `crates/worldwake-systems/src/patrol.rs`
- `crates/worldwake-systems/src/tell_actions.rs`
- `crates/worldwake-systems/src/justice_actions.rs`
- `crates/worldwake-systems/tests/e15_information_integration.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`
- `crates/worldwake-ai/tests/conformance_execution_budget.rs`
- `crates/worldwake-ai/tests/golden_activation_decay.rs`
- `crates/worldwake-ai/tests/golden_ai_decisions.rs`
- `crates/worldwake-ai/tests/golden_experience_preferences.rs`
- `crates/worldwake-ai/tests/golden_exploration.rs`
- `crates/worldwake-ai/tests/golden_harness/mod.rs`
- `crates/worldwake-ai/tests/golden_harness/soak_world.rs`
- `crates/worldwake-ai/tests/golden_offices.rs`
- `crates/worldwake-ai/tests/golden_planner_pathology.rs`
- `crates/worldwake-ai/tests/golden_simulation_gaps.rs`
- `crates/worldwake-cli/src/scenario/mod.rs`
- `crates/worldwake-cli/src/scenario/types.rs`

### 3. Extend focused proofs

- extend `worldwake-core` tests to prove `Default`, omitted-field serde defaulting, and explicit-value serde parsing
- extend `worldwake-cli` scenario tests to prove explicit authored `observation_budget` input and omitted-field defaulting

## Files to Touch

- `crates/worldwake-core/src/belief.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-systems/src/perception.rs`
- `crates/worldwake-systems/src/patrol.rs`
- `crates/worldwake-systems/src/tell_actions.rs`
- `crates/worldwake-systems/src/justice_actions.rs`
- `crates/worldwake-systems/tests/e15_information_integration.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`
- `crates/worldwake-ai/tests/conformance_execution_budget.rs`
- `crates/worldwake-ai/tests/golden_activation_decay.rs`
- `crates/worldwake-ai/tests/golden_ai_decisions.rs`
- `crates/worldwake-ai/tests/golden_experience_preferences.rs`
- `crates/worldwake-ai/tests/golden_exploration.rs`
- `crates/worldwake-ai/tests/golden_harness/mod.rs`
- `crates/worldwake-ai/tests/golden_harness/soak_world.rs`
- `crates/worldwake-ai/tests/golden_offices.rs`
- `crates/worldwake-ai/tests/golden_planner_pathology.rs`
- `crates/worldwake-ai/tests/golden_simulation_gaps.rs`
- `crates/worldwake-cli/src/scenario/mod.rs`
- `crates/worldwake-cli/src/scenario/types.rs`

## Out of Scope

- Observation priority logic or pipeline modification (S105OBSSALFIL-002)
- Any behavioral change beyond storing/defaulting the new field
- Modifying existing `.ron` scenario files
- Goal-specific observation filtering
- Dynamic budget adjustment

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace --no-run`
2. focused `worldwake-core` default/serde proof for `PerceptionProfile`
3. focused `worldwake-cli` authored-input/defaulting proof
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `PerceptionProfile::default().observation_budget == 24`
2. Existing scenario-authored `perception_profile` blocks deserialize without an explicit `observation_budget`
3. Remaining exhaustive non-archive `PerceptionProfile { ... }` literals compile with the new field present

## Test Plan

### New/Modified Tests

1. Extend existing `worldwake-core` tests for `PerceptionProfile` defaulting and serde behavior
2. Extend existing `worldwake-cli` scenario tests for explicit and omitted `observation_budget`

### Commands

1. `cargo test --workspace --no-run`
2. `cargo test -p worldwake-core --lib belief::tests::perception_profile_default_includes_activation_decay_fields -- --exact`
3. `cargo test -p worldwake-core --lib belief::tests::perception_profile_serde_defaults_observation_budget_when_omitted -- --exact`
4. `cargo test -p worldwake-core --lib belief::tests::perception_profile_serde_accepts_explicit_observation_budget -- --exact`
5. `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_deserialize_full -- --exact`
6. `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_perception_profile_defaults_observation_budget_when_omitted -- --exact`
7. `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_agent_with_profile_overrides -- --exact`
8. `cargo test --workspace`
9. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-16.

- Added `observation_budget: u8` to `PerceptionProfile` with a concrete serde default helper returning `24`.
- Updated the real exhaustive `PerceptionProfile` literals that compile-failed after the shared-field addition; call sites already using `..PerceptionProfile::default()` were left unchanged.
- Extended focused proof in `worldwake-core` and `worldwake-cli` so the ticket now proves both omitted-field compatibility and explicit authored input.

## Deviations

- The draft ticket overstated the constructor fallout and originally treated this as a build-only verification ticket. Reassessment narrowed the real edit surface and added focused serde/authored-input proof instead of editing unchanged default-spread call sites.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-core --lib belief::tests::perception_profile_default_includes_activation_decay_fields -- --exact`
- Passed `cargo test -p worldwake-core --lib belief::tests::perception_profile_serde_defaults_observation_budget_when_omitted -- --exact`
- Passed `cargo test -p worldwake-core --lib belief::tests::perception_profile_serde_accepts_explicit_observation_budget -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_deserialize_full -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_perception_profile_defaults_observation_budget_when_omitted -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_agent_with_profile_overrides -- --exact`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
