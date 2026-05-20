# S152COGARCSEE-004: Scenario integration — archetype policy and per-agent override fields

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — scenario definition fields and authored-field coverage metadata (`worldwake-cli`)
**Deps**: archive/tickets/S152COGARCSEE-001.md

## Problem

Scenario authors need to control archetype assignment: a per-scenario policy (uniform default-five, authored uniform set, or frequency-weighted) and a per-agent override that pins a specific agent's archetype. S152 adds `ScenarioDef.archetype_assignment_policy` and `AgentDef.archetype`, both optional with serde defaults so existing scenarios continue to load.

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ScenarioDef` (`crates/worldwake-cli/src/scenario/types.rs`) already has `pub seed: u64` and many `#[serde(default)]` optional fields. `AgentDef` (`crates/worldwake-cli/src/scenario/types.rs`) carries per-agent profile overrides as `Option<…>` with `#[serde(default)]` (e.g. `cognitive_profile: Option<CognitiveProfile>`). The new fields follow this exact pattern.
2. `ArchetypeAssignmentPolicy` and `CognitiveArchetype` are defined in `worldwake-core` (ticket 001) and re-exported; `worldwake-cli` depends on core. No `RoleTag`/`AgentName` types are referenced — those were removed during reassessment (they do not exist; agent names are `String`).
3. Boundary under audit: RON deserialization of `ScenarioDef`/`AgentDef`. Because both new fields use `#[serde(default)]`, existing `scenarios/**/*.ron` files that omit them still deserialize — no scenario-file edits are required (confirmed by the serde-default contract; the new fields are additive `Option`s).
4. (Mismatch + correction) `ScenarioDef` is the scenario *input* definition (RON), not the saved `SimulationState`; adding fields here does **not** bump `SAVE_FORMAT_VERSION`. Only tickets 002/003 (which mutate serialized world/event state) bump the save format.
5. Implementation exposed same-crate and downstream explicit struct-literal fallout plus the `scenario-coverage` authored-field inventory. These were current-ticket compile/coverage fallout of adding public scenario schema fields, not runtime resolution work.

## Architecture Check

1. Mirroring the existing optional-profile-override idiom on `AgentDef` keeps per-agent archetype authoring consistent with how every other agent profile is authored, and avoids inventing a name-keyed policy map that would duplicate per-agent authoring (the reason `Explicit`/`PerRole` were dropped in reassessment).
2. No backwards-compatibility shim: `#[serde(default)]` is the standard additive-field mechanism, not a compatibility layer; absent fields resolve to the default policy at spawn (ticket 005).

## Verified Layers

1. RON with no archetype fields deserializes -> existing minimal scenario-definition unit test now asserts both fields default to `None`.
2. RON specifying a weighted policy and a per-agent `archetype` deserializes into the expected values -> added `scenario::types::tests::test_scenario_def_deserializes_archetype_policy_and_agent_override`.
3. Authored-field coverage recognizes archetype policy/override fields -> `scenario-coverage` bin unit tests and `worldwake-cli` crate tests.
4. Single-layer ticket (scenario-def deserialization only); resolution behavior remains ticket 005, so no decision/action-trace layer applies.

## Landed Changes

### 1. `ScenarioDef.archetype_assignment_policy`

Added `#[serde(default)] pub archetype_assignment_policy: Option<ArchetypeAssignmentPolicy>` to `ScenarioDef` (`types.rs`).

### 2. `AgentDef.archetype`

Added `#[serde(default)] pub archetype: Option<CognitiveArchetype>` to `AgentDef` (`types.rs`).

### 3. Explicit constructor and coverage fallout

Updated explicit `ScenarioDef`/`AgentDef` literals in CLI tests/helpers and one `worldwake-ai` scenario test helper with `None` values. Mapped authored archetype fields in `scenario-coverage` so future authored usage is visible in the feature inventory rather than silently ignored.

## Landed Files

- `crates/worldwake-cli/src/scenario/types.rs` — two serde-default fields plus focused RON tests.
- `crates/worldwake-cli/src/bin/scenario_coverage.rs` — authored-field mapping for archetype policy/override.
- `crates/worldwake-cli/src/display.rs`, `crates/worldwake-cli/src/handlers/*.rs`, `crates/worldwake-cli/src/scenario/lints.rs`, `crates/worldwake-cli/src/scenario/mod.rs` — explicit test/helper constructor fallout.
- `crates/worldwake-ai/tests/scenarios/survival_baseline.rs` — downstream explicit `ScenarioDef` literal fallout.

## Out of Scope

- Drawing/resolving the archetype or applying deltas (ticket 005).
- Any change to existing `scenarios/**/*.ron` files (serde defaults make edits unnecessary).
- Per-role or name-keyed policy variants (Non-Goal — deferred to a future sibling spec).

## Acceptance Result

### Tests Passed

1. Passed: a `ScenarioDef` RON omitting both fields deserializes with `archetype_assignment_policy == None` and each `AgentDef.archetype == None`.
2. Passed: a RON specifying `archetype_assignment_policy: Weighted({...})` and an agent with `archetype: Bold` deserializes to those values.
3. Passed: existing `worldwake-cli` suite.

### Invariants

1. Existing scenarios load unchanged through additive `#[serde(default)]` fields.
2. No `SAVE_FORMAT_VERSION` change was made; scenario definition is authored input, not saved simulation state.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-cli/src/scenario/types.rs` — added weighted policy/per-agent override RON deserialization coverage; extended minimal deserialization assertions for omitted fields.
2. `crates/worldwake-cli/src/bin/scenario_coverage.rs` — existing bin-local tests cover the new feature mapping.

### Commands Run

1. Passed `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_deserializes_archetype_policy_and_agent_override -- --exact`
2. Passed `cargo test -p worldwake-cli scenario`
3. Passed `cargo test -p worldwake-cli --bin scenario-coverage`
4. Passed `cargo test -p worldwake-cli`
5. Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
6. Passed `cargo test --workspace --no-run`
7. `./scripts/verify.sh` not run for this ticket iteration; the implement-spec harness reserves it for final pre-push verification after the full S152 ticket family lands.

## Outcome

Completed on 2026-05-20.

- Added scenario-authored archetype policy and per-agent archetype override fields with serde defaults.
- Added focused RON coverage for omitted fields, weighted policy parsing, and per-agent override parsing.
- Updated explicit constructors and the scenario-coverage authored-field inventory required by the new public schema fields.
- No save-format version change was made.

## Deviations

- The drafted file list named only `types.rs`, but live compile and coverage checks required explicit constructor fallout across CLI tests/helpers, one downstream `worldwake-ai` test helper, and `scenario_coverage`.
- Runtime archetype assignment and profile resolution remain owned by `tickets/S152COGARCSEE-005.md`.

## Verification Result

- Passed `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_deserializes_archetype_policy_and_agent_override -- --exact`
- Passed `cargo test -p worldwake-cli scenario`
- Passed `cargo test -p worldwake-cli --bin scenario-coverage`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
- Passed `cargo test --workspace --no-run`
- Waived `./scripts/verify.sh` for this ticket iteration because the harness runs the full pre-push gate after the whole S152 family lands.
