# S97POSNOTART-004: CLI scenario support for `ArtifactPostingProfile`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — CLI/scenario infrastructure only
**Deps**: archive/tickets/S97POSNOTART-001.md

## Problem

The scenario profile completeness invariant requires every universal agent component to be configurable via `AgentDef` and applied in `spawn_agent()`. Without this, scenario authors cannot customize per-agent TTL values (e.g., guards posting longer-lived warnings than civilians).

## Assumption Reassessment (2026-04-12)

1. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:67` has ~35 optional profile fields. No `ArtifactPostingProfile` or similar field exists. The field should follow the existing `Option<ProfileType>` pattern used by other directly authorable universal profiles.
2. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:323` already applies several universal profiles with `unwrap_or_default()` (`perception_profile`, `tell_profile`, `cognitive_profile`, `execution_budget`, `epistemic_disposition`, `intention_disposition`, `communication_profile`, `preference_profile`, `expectation_store`, `last_seen_memory`, `obligation_satiation_profile`). `ArtifactPostingProfile` belongs on that same universal path.
3. `ArtifactPostingProfile` contains no `EntityId` references — no scenario-specific `*Def` wrapper is needed. It can be deserialized directly in `AgentDef`.
4. The crate already contains the honest focused proof surfaces this ticket needs:
   - `crates/worldwake-cli/src/scenario/types.rs`: deserialize tests, including omitted-field/default-path coverage
   - `crates/worldwake-cli/src/scenario/mod.rs`: spawn-time universal-profile assertions on the authoritative world
   This ticket should update those existing tests rather than relying only on broad `cargo test -p worldwake-cli`.
5. Existing RON scenarios do not reference artifact posting and should continue to load through the new `Option` field defaulting to `None`.
6. Adding a new `AgentDef` field also creates same-crate constructor fallout in other `worldwake-cli` scenario builders and handler/display tests that still use full manual `AgentDef` literals. Those literals need `artifact_posting_profile: None` to keep the scenario authoring surface compiling.

## Architecture Check

1. Universal component with `unwrap_or_default()` is the established pattern — consistent with `CognitiveProfile`, `PerceptionProfile`, `ExplorationProfile`, etc.
2. No backward-compatibility shims — existing scenarios compile without changes since the new field is `Option` and defaults to `None` (triggering `unwrap_or_default()`).

## Verification Layers

1. Scenario with explicit profile loads correctly → focused `AgentDef` RON deserialize test
2. Scenario without profile gets default values → focused omitted-field/default-path deserialize test plus spawn-time universal-profile assertion
3. Spawned agents receive default/override `ArtifactPostingProfile` values through the authoritative world component store
4. Same-crate manual `AgentDef` builders continue compiling after the new field lands
5. Single-layer ticket (CLI scenario infrastructure) — no simulation-layer behavior change needed.

## What to Change

### 1. Add field to `AgentDef`

In `crates/worldwake-cli/src/scenario/types.rs`, add:

```rust
pub artifact_posting_profile: Option<ArtifactPostingProfile>,
```

Import `ArtifactPostingProfile` from `worldwake-core`.

### 2. Add registration in `spawn_agent()`

In `crates/worldwake-cli/src/scenario/mod.rs`, add after the existing universal profile registrations:

```rust
txn.set_component_artifact_posting_profile(
    agent,
    agent_def.artifact_posting_profile.clone().unwrap_or_default(),
)?;
```

### 3. Update existing focused scenario tests

Use the crate's existing proof surfaces instead of relying on broad suite-only verification:
- add deserialize coverage in `scenario/types.rs` for explicit `artifact_posting_profile` input and omitted-field/default behavior
- extend `scenario/mod.rs` spawn-time assertions so default universal-profile coverage includes `ArtifactPostingProfile`
- add a focused override assertion in `scenario/mod.rs` proving an authored profile survives `spawn_scenario()` into authoritative world state

### 4. Update same-crate manual `AgentDef` literals

Add `artifact_posting_profile: None` to existing full `AgentDef` literals across `worldwake-cli` handler/display tests and scenario builders that do not use `..minimal_agent(...)`, so the scenario authoring surface remains exhaustive and compiles cleanly after the new field lands.

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — add field to `AgentDef` + focused deserialize tests)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — add `set_component` call in `spawn_agent` + focused spawn assertions)
- `crates/worldwake-cli/src/display.rs` (modify — same-crate manual `AgentDef` literals)
- `crates/worldwake-cli/src/handlers/actions.rs` (modify — same-crate manual `AgentDef` literals)
- `crates/worldwake-cli/src/handlers/control.rs` (modify — same-crate manual `AgentDef` literals)
- `crates/worldwake-cli/src/handlers/events.rs` (modify — same-crate manual `AgentDef` literals)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify — same-crate manual `AgentDef` literals)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify — same-crate manual `AgentDef` literals)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify — same-crate manual `AgentDef` literals)

## Out of Scope

- Updating existing RON scenario files to include explicit posting profiles (they get defaults)
- GoalBeliefView accessor (ticket 002)
- Candidate generation changes (ticket 003)
- Golden tests (ticket 005)

## Acceptance Criteria

### Tests That Must Pass

1. Scenario with explicit `artifact_posting_profile` field in RON deserializes correctly
2. Scenario without the field keeps `artifact_posting_profile: None` at `AgentDef` deserialize time
3. `spawn_scenario()` applies `ArtifactPostingProfile::default()` when the field is omitted
4. `spawn_scenario()` preserves an authored `ArtifactPostingProfile` override
5. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Every agent spawned via `spawn_agent()` has an `ArtifactPostingProfile`
2. Existing scenarios continue to load without modification
3. Existing same-crate manual `AgentDef` builders remain exhaustive and compile after the new field lands

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/types.rs` — add explicit and omitted-field deserialize assertions for `artifact_posting_profile`
2. `crates/worldwake-cli/src/scenario/mod.rs` — extend default universal-profile proof and add authored override proof for `ArtifactPostingProfile`
3. Existing same-crate handler/display scenario builders compile with the new `AgentDef` field present

### Commands

1. `cargo test -p worldwake-cli test_scenario_def_artifact_posting_profile_deserializes_when_present`
2. `cargo test -p worldwake-cli test_scenario_def_artifact_posting_profile_omitted_field_stays_none`
3. `cargo test -p worldwake-cli test_spawn_agents_receive_default_universal_profiles`
4. `cargo test -p worldwake-cli test_spawn_agent_with_artifact_posting_profile_override`
5. `cargo test -p worldwake-cli`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed as a CLI scenario-authoring slice. `AgentDef` now supports an optional `artifact_posting_profile`, `spawn_agent()` applies `ArtifactPostingProfile::default()` when omitted and preserves authored overrides, and the existing focused scenario deserialize/spawn tests now prove both the omitted-field path and the explicit profile path.

## Deviations

1. The original draft understated same-crate fallout. Because `AgentDef` is built through full manual literals in several `worldwake-cli` handler/display test scenario builders, broadened verification required adding `artifact_posting_profile: None` to those builders so the scenario authoring surface stayed exhaustive and compiling.
2. The original draft claimed no focused tests needed modification. Reassessment corrected that to the live proof surface already present in `scenario/types.rs` and `scenario/mod.rs`.

## Verification Result

1. Passed `cargo test -p worldwake-cli test_scenario_def_artifact_posting_profile_deserializes_when_present`
2. Passed `cargo test -p worldwake-cli test_scenario_def_artifact_posting_profile_omitted_field_stays_none`
3. Passed `cargo test -p worldwake-cli test_spawn_agents_receive_default_universal_profiles`
4. Passed `cargo test -p worldwake-cli test_spawn_agent_with_artifact_posting_profile_override`
5. Passed `cargo test -p worldwake-cli`
6. Passed `cargo clippy --workspace --all-targets -- -D warnings`
