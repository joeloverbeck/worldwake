# S130SURRECFRO-001: Profile field additions for survey memory

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `CognitiveProfile`, `ExplorationProfile`, `ExplorationProfileDef`, `SAVE_FORMAT_VERSION`
**Deps**: spec `specs/S130-survey-records-frontier-disconfirmation.md` D9

## Problem

S130 introduces per-agent survey memory and ranking damping. Both need configuration fields the existing profile components don't carry: a capacity and retention window for `SurveyMemory` (cognition) and a damping window and strength for negative-survey ranking (exploration). Without these fields landing first, downstream tickets reference fields that don't yet compile.

## Assumption Reassessment (2026-05-02)

1. `CognitiveProfile` exists at `crates/worldwake-core/src/cognitive_profile.rs`; no `*Def` mirror — `AgentDef.cognitive_profile: Option<CognitiveProfile>` at `crates/worldwake-cli/src/scenario/types.rs` consumes the core type directly. Adding `#[serde(default = "...")]` on new fields preserves compatibility for existing scenarios with `cognitive_profile:` blocks.
2. `ExplorationProfile` exists at `crates/worldwake-core/src/exploration.rs`; `ExplorationProfileDef` mirror exists at `crates/worldwake-cli/src/scenario/types.rs` and is the scenario-authoring surface. The live branch had inline spawn conversion rather than `From`/`Into` impls, so this ticket added the conversion impls while adding the new fields.
3. Spec `specs/S130-survey-records-frontier-disconfirmation.md` D9 mandates asymmetric handling — `CognitiveProfile` takes fields directly with serde defaults (no mirror); `ExplorationProfileDef` mirror gets matching field additions. The asymmetry reflects existing convention: `CognitiveProfile` has no `EntityId` references requiring `*Def` indirection.
4. `CognitiveProfile` and `ExplorationProfile` are save-bound component payloads. Adding non-skipped fields changes the current-format bincode shape, so this ticket owns the `SAVE_FORMAT_VERSION` bump from `58` to `59`; ticket 004 must now treat `59` as its baseline.

## Architecture Check

1. Field placement is symmetric with sibling profile components: capacity and retention belong on cognitive (alongside `repair_memory_ticks`, `learned_opportunity_memory_ticks`); damping window and strength belong on exploration (alongside `curiosity_weight`, `frontier_depth`).
2. No backward-compatibility shims — fields are net-new with `#[serde(default)]` for scenario deserialization; current save format is bumped rather than migrated.
3. Profile fields are concrete per-agent state (FND-22 / FND-22A); no abstract scoring or magic numbers in agent-side code (FND-3 / no-magic-numbers convention).

## Verification Layers

1. Default values match spec → focused unit tests asserting `CognitiveProfile::default().survey_memory_capacity == 24`, `survey_memory_retention_ticks == 300`, `ExplorationProfile::default().negative_survey_damping_window == 200`, `negative_survey_damping_strength == Permille::new_unchecked(800)`.
2. Existing scenarios deserialize unchanged → focused unit test deserializing a sample `cognitive_profile:` and `exploration_profile:` RON block without the new fields and confirming defaults populate.
3. `ExplorationProfileDef` ↔ `ExplorationProfile` round-trip preserves new fields → focused unit test on the From/Into impls.
4. Save format is bumped and save/load round-trip proof covers the changed persisted profile shape.

## What to Change

### 1. `CognitiveProfile` field additions

In `crates/worldwake-core/src/cognitive_profile.rs`, add two fields to `CognitiveProfile` with serde defaults:

```rust
#[serde(default = "default_survey_memory_capacity")]
pub survey_memory_capacity: usize,
#[serde(default = "default_survey_memory_retention_ticks")]
pub survey_memory_retention_ticks: u64,
```

Define `fn default_survey_memory_capacity() -> usize { 24 }` and `fn default_survey_memory_retention_ticks() -> u64 { 300 }` alongside other default helpers in the same module. Update the `Default` impl to populate these constants. No `*Def` mirror update — `AgentDef.cognitive_profile` already uses `Option<CognitiveProfile>` directly.

### 2. `ExplorationProfile` field additions

In `crates/worldwake-core/src/exploration.rs`, add two fields to `ExplorationProfile` with serde defaults:

```rust
#[serde(default = "default_negative_survey_damping_window")]
pub negative_survey_damping_window: u32,
#[serde(default = "default_negative_survey_damping_strength")]
pub negative_survey_damping_strength: Permille,
```

Define `fn default_negative_survey_damping_window() -> u32 { 200 }` and `fn default_negative_survey_damping_strength() -> Permille { Permille::new_unchecked(800) }`. Update the `Default` impl with the same constants.

### 3. `ExplorationProfileDef` mirror update

In `crates/worldwake-cli/src/scenario/types.rs`, add the matching fields to `ExplorationProfileDef` with the same serde defaults. Update the `From<ExplorationProfileDef> for ExplorationProfile` (and the inverse if present) impls to round-trip both new fields.

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify)
- `crates/worldwake-core/src/exploration.rs` (modify)
- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — use `From<ExplorationProfileDef>`)
- `crates/worldwake-cli/src/scenario/lints.rs` (modify — test fixture literal)
- `crates/worldwake-core/src/delta.rs` (modify — representative `ComponentValue` literal)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — test fixture literal)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` 58→59)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — test helper literal)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — test helper literal)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — test helper literal)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — test helper literal)
- `crates/worldwake-ai/src/goal_model.rs` (modify — test helper literal)
- `crates/worldwake-ai/src/search/tests.rs` (modify — test helper literal)

## Out of Scope

- `SurveyMemory` component definition and its `enforce_limits` body (ticket 002)
- Calling `enforce_limits` from a SystemFn (ticket 008)
- Reading these fields in ranking or perception (tickets 006, 007)
- Adding a `CognitiveProfileDef` mirror — explicitly rejected by spec D9; the existing direct `Option<CognitiveProfile>` path stands

## Acceptance Criteria

### Tests That Must Pass

1. New: `cognitive_profile_default_includes_survey_memory_fields` — asserts default values match spec.
2. New: `exploration_profile_default_includes_negative_survey_damping_fields` — asserts default values match spec.
3. New: `exploration_profile_def_round_trips_negative_survey_damping_fields` — asserts From/Into preserve new fields.
4. New: `cognitive_profile_deserializes_without_new_fields_using_serde_defaults` — asserts existing RON blocks deserialize and populate defaults.
5. Existing suite: `cargo test -p worldwake-core`.
6. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. New fields have explicit `#[serde(default)]` so existing serialized scenarios deserialize without modification. Save format is bumped to `59` because these profiles are persisted component payloads.
2. `Default` impl values are concrete constants matching the spec — no magic numbers leak into agent-side reasoning code.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_profile.rs` (`#[cfg(test)]` block) — existing default/serde/bincode tests extended for survey fields.
2. `crates/worldwake-core/src/exploration.rs` (`#[cfg(test)]` block) — existing default/bincode tests extended for damping fields.
3. `crates/worldwake-cli/src/scenario/types.rs` (`#[cfg(test)]` block) — new ExplorationProfileDef serde-default and round-trip tests; existing scenario omitted-field tests extended.
4. `crates/worldwake-sim/src/save_load.rs` (`#[cfg(test)]` block) — version assertion updated to `59`.

### Commands

1. `cargo test -p worldwake-core cognitive_profile`
2. `cargo test -p worldwake-core exploration`
3. `cargo test -p worldwake-cli`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-02.

- Added `CognitiveProfile.survey_memory_capacity` and `survey_memory_retention_ticks` with serde defaults and `Default` values from S130 D9.
- Added `ExplorationProfile.negative_survey_damping_window` and `negative_survey_damping_strength` with serde defaults and mirrored them through `ExplorationProfileDef`.
- Added `Default`, `From<ExplorationProfileDef> for ExplorationProfile`, and `From<ExplorationProfile> for ExplorationProfileDef`; `spawn_agent` now uses that conversion instead of an inline literal.
- Updated explicit profile literals and fixtures surfaced by the all-target compile sweep.
- Bumped `SAVE_FORMAT_VERSION` from `58` to `59` because the profile fields change persisted component payload shape.

## Deviations

- The draft said the save-format bump would live in ticket 004. Reassessment showed this ticket is the first persisted-shape change in S130, so the bump landed here. Ticket 004 now owns the next bump from `59` when `SurveyMemory` registration changes saved agent component state.
- The live `ExplorationProfileDef` path did not have pre-existing `From`/`Into` impls; this ticket added them and switched scenario spawn to that seam.
- The focused assertions landed by extending existing live profile tests and adding scenario-type tests under their final names, rather than preserving the draft placeholder test names verbatim.

## Verification Result

- Passed `cargo test -p worldwake-core --lib cognitive_profile::tests::cognitive_profile_default_matches_split_defaults -- --exact`
- Passed `cargo test -p worldwake-core --lib cognitive_profile::tests::cognitive_profile_deserialization_defaults_memory_ttls -- --exact`
- Passed `cargo test -p worldwake-core --lib exploration::tests::exploration_profile_default_matches_spec_defaults -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::types::tests::exploration_profile_def_round_trips_negative_survey_damping_fields -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::types::tests::exploration_profile_def_defaults_negative_survey_damping_fields_when_omitted -- --exact`
- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-core cognitive_profile`
- Passed `cargo test -p worldwake-core exploration`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-sim save_load`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `./scripts/verify.sh` (live gates: `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p worldwake-cli --bin scenario-coverage -- --check`)
