# S102FROAWAEXP-001: Add ExplorationProfile new fields + scenario support

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — ExplorationProfile struct (worldwake-core), ExplorationProfileDef (worldwake-cli), spawn_agent conversion
**Deps**: S102 spec

## Problem

Agents cannot configure per-agent frontier depth, acquisition failure threshold, or exploration arrival boost because ExplorationProfile lacks these fields. All downstream S102 tickets depend on these parameters existing.

## Assumption Reassessment (2026-04-14)

1. `ExplorationProfile` exists at `crates/worldwake-core/src/exploration.rs:6-12` with 5 fields: `curiosity_weight`, `need_activation_threshold`, `max_consecutive_explorations`, `visit_lookback_ticks`, `consecutive_exploration_count`. Derives: Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize. Default impl at lines 14-24. All new fields are `Copy` types — struct remains `Copy`.
2. `ExplorationProfileDef` exists at `crates/worldwake-cli/src/scenario/types.rs:167-172` with 4 fields (excludes runtime `consecutive_exploration_count`). Field in `AgentDef` at line 113 as `Option<ExplorationProfileDef>`.
3. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:348-358` explicitly enumerates all ExplorationProfile fields with `consecutive_exploration_count: 0`. New fields must be added to both the Def struct and the conversion block.
4. ExplorationProfile struct literal construction sites: 27 across 14 files. Most use `Default::default()` or spread syntax. Key explicit-enumeration sites: `scenario/mod.rs` (2), `exploration.rs` Default impl (1), `delta.rs` macro expansion (1). Test fixtures in `candidate_generation.rs` and golden tests use `ExplorationProfile { ... }` but typically with spread.
5. Existing authored input already uses `exploration_profile` in `scenarios/cli-evaluation.ron`, and scenario parsing tests deserialize an `ExplorationProfileDef` without the new S102 fields. The new scenario-facing fields therefore need field-level serde defaults so current authored profiles remain valid while the new overrides become available.

## Architecture Check

1. Adding fields to an existing profile with defaults is the simplest approach — no new types, no trait changes, no new registration. `frontier_depth: 2` preserves existing behavior (1-hop was the effective depth; 2-hop enables the new feature).
2. No backward-compatibility shims in runtime behavior. The old single-hop behavior is superseded when `frontier_depth >= 2`.
3. Scenario-facing serde defaults are required for additive authored-input compatibility on the current branch because existing RON already authors `exploration_profile` without the new fields.

## Verification Layers

1. New fields accessible from `ExplorationProfile` → focused unit test confirming Default values
2. Scenario-definable via RON → integration test or manual scenario load confirming field override
3. Existing authored RON that omits the new fields still deserializes to the S102 defaults
4. Single-layer ticket (core types + scenario bootstrap) — no behavioral AI/planner verification needed

## What to Change

### 1. Add three fields to ExplorationProfile

In `crates/worldwake-core/src/exploration.rs`:

- Add `pub frontier_depth: u16` (default: 2)
- Add `pub acquisition_failure_threshold: u8` (default: 3)
- Add `pub exploration_arrival_boost: Permille` (default: `Permille::new_unchecked(500)`)
- Update `Default` impl with these values

### 2. Update ExplorationProfileDef

In `crates/worldwake-cli/src/scenario/types.rs`:

- Add corresponding fields to `ExplorationProfileDef`: `frontier_depth: u16`, `acquisition_failure_threshold: u8`, `exploration_arrival_boost: Permille`
- Add field-level serde defaults so existing authored `exploration_profile` blocks remain valid when they omit the new S102 fields

### 3. Update spawn_agent conversion

In `crates/worldwake-cli/src/scenario/mod.rs`:

- Add new fields to the `ExplorationProfile { ... }` construction in `spawn_agent()`, reading from `ExplorationProfileDef`

### 4. Fix construction sites

Update any explicit field-enumeration sites that fail to compile after the field addition:
- `crates/worldwake-core/src/delta.rs` (macro expansion)
- Test fixtures in `crates/worldwake-ai/` that construct `ExplorationProfile` without spread syntax

## Files to Touch

- `crates/worldwake-core/src/exploration.rs` (modify)
- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify — macro expansion)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — explicit fixture construction)
- Test fixtures in `crates/worldwake-ai/src/candidate_generation.rs` (modify — explicit construction)
- Golden fixtures in `crates/worldwake-ai/tests/golden_exploration.rs` (modify — explicit construction)
- Golden fixtures in `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify — explicit construction)
- Golden fixtures in `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` (modify — explicit construction)

## Out of Scope

- AcquisitionExhaustionTracker component (ticket 002)
- Any behavioral changes to candidate generation or target selection
- Modifying S101 decay rates or S100 retention windows
- Need-directed exploration targeting

## Acceptance Criteria

### Tests That Must Pass

1. `ExplorationProfile::default()` returns `frontier_depth: 2`, `acquisition_failure_threshold: 3`, `exploration_arrival_boost: Permille(500)`
2. Scenario-authored `exploration_profile` values can omit the new S102 fields and still deserialize to those defaults
3. Scenario-authored overrides for the new fields populate `ExplorationProfile` during `spawn_agent()`
4. Workspace builds cleanly: `cargo build --workspace`
5. Existing suite: `cargo test --workspace`

### Invariants

1. `ExplorationProfile` remains `Copy` — all fields are `Copy` types
2. All existing tests pass without behavioral changes (new fields have defaults that preserve existing behavior)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/exploration.rs` — unit test for Default values of new fields
2. `crates/worldwake-cli/src/scenario/types.rs` — unit test proving omitted authored fields default correctly
3. `crates/worldwake-cli/src/scenario/mod.rs` — unit test proving authored overrides reach spawned `ExplorationProfile`
4. Modified fixtures in sim/AI tests that explicitly construct `ExplorationProfile`

### Commands

1. `cargo test -p worldwake-core --lib exploration::tests::exploration_profile_default_matches_spec_defaults -- --exact`
2. `cargo test -p worldwake-cli --lib scenario::types::tests::test_exploration_profile_def_defaults_new_fields_when_omitted -- --exact`
3. `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_agent_with_profile_overrides -- --exact`
4. `cargo build --workspace`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-14.

- Added `frontier_depth`, `acquisition_failure_threshold`, and `exploration_arrival_boost` to `ExplorationProfile` with the S102 defaults and updated the core default/bincode/sample coverage.
- Extended `ExplorationProfileDef` and `spawn_agent()` so authored scenarios can override the new fields while runtime-only `consecutive_exploration_count` still starts at `0`.
- Added field-level serde defaults for the new scenario-facing fields so existing authored `exploration_profile` blocks, including `scenarios/cli-evaluation.ron`-style input, remain valid when those fields are omitted.
- Updated explicit `ExplorationProfile` fixtures across core/sim/AI tests to match the widened struct shape.

## Deviations

- The draft ticket treated the scenario-facing change as a straight additive field landing. Live reassessment showed existing authored RON already uses `exploration_profile`, so this ticket also had to own field-level serde defaults on `ExplorationProfileDef` to preserve lawful current authored input.

## Verification Result

- Passed `cargo test -p worldwake-core --lib exploration::tests::exploration_profile_default_matches_spec_defaults -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::types::tests::test_exploration_profile_def_defaults_new_fields_when_omitted -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_agent_with_profile_overrides -- --exact`
- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
