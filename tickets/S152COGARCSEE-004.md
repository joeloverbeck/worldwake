# S152COGARCSEE-004: Scenario integration — archetype policy and per-agent override fields

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — scenario definition fields (`worldwake-cli`)
**Deps**: S152COGARCSEE-001

## Problem

Scenario authors need to control archetype assignment: a per-scenario policy (uniform default-five, authored uniform set, or frequency-weighted) and a per-agent override that pins a specific agent's archetype. S152 adds `ScenarioDef.archetype_assignment_policy` and `AgentDef.archetype`, both optional with serde defaults so existing scenarios continue to load.

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ScenarioDef` (`crates/worldwake-cli/src/scenario/types.rs:31`) already has `pub seed: u64` (`:31`) and many `#[serde(default)]` optional fields. `AgentDef` (`types.rs:573`) carries per-agent profile overrides as `Option<…>` with `#[serde(default)]` (e.g. `cognitive_profile: Option<CognitiveProfile>`). The new fields follow this exact pattern.
2. `ArchetypeAssignmentPolicy` and `CognitiveArchetype` are defined in `worldwake-core` (ticket 001) and re-exported; `worldwake-cli` depends on core. No `RoleTag`/`AgentName` types are referenced — those were removed during reassessment (they do not exist; agent names are `String`).
3. Boundary under audit: RON deserialization of `ScenarioDef`/`AgentDef`. Because both new fields use `#[serde(default)]`, existing `scenarios/**/*.ron` files that omit them still deserialize — no scenario-file edits are required (confirmed by the serde-default contract; the new fields are additive `Option`s).
4. (Mismatch + correction) `ScenarioDef` is the scenario *input* definition (RON), not the saved `SimulationState`; adding fields here does **not** bump `SAVE_FORMAT_VERSION`. Only tickets 002/003 (which mutate serialized world/event state) bump the save format.

## Architecture Check

1. Mirroring the existing optional-profile-override idiom on `AgentDef` keeps per-agent archetype authoring consistent with how every other agent profile is authored, and avoids inventing a name-keyed policy map that would duplicate per-agent authoring (the reason `Explicit`/`PerRole` were dropped in reassessment).
2. No backwards-compatibility shim: `#[serde(default)]` is the standard additive-field mechanism, not a compatibility layer; absent fields resolve to the default policy at spawn (ticket 005).

## Verification Layers

1. RON with no archetype fields deserializes -> focused scenario-loader unit test (authoritative scenario-def state).
2. RON specifying a policy and a per-agent `archetype` deserializes into the expected values -> focused scenario-loader unit test.
3. Single-layer ticket (scenario-def deserialization only); resolution behavior is ticket 005, so no decision/action-trace layer applies.

## What to Change

### 1. `ScenarioDef.archetype_assignment_policy`

Add `#[serde(default)] pub archetype_assignment_policy: Option<ArchetypeAssignmentPolicy>` to `ScenarioDef` (`types.rs`).

### 2. `AgentDef.archetype`

Add `#[serde(default)] pub archetype: Option<CognitiveArchetype>` to `AgentDef` (`types.rs`).

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — two fields + import)

## Out of Scope

- Drawing/resolving the archetype or applying deltas (ticket 005).
- Any change to existing `scenarios/**/*.ron` files (serde defaults make edits unnecessary).
- Per-role or name-keyed policy variants (Non-Goal — deferred to a future sibling spec).

## Acceptance Criteria

### Tests That Must Pass

1. A `ScenarioDef` RON omitting both new fields deserializes with `archetype_assignment_policy == None` and each `AgentDef.archetype == None`.
2. A RON specifying `archetype_assignment_policy: Some(Weighted({...}))` and an agent with `archetype: Some(Bold)` deserializes to those values.
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Existing scenarios load unchanged (additive `#[serde(default)]` fields).
2. No `SAVE_FORMAT_VERSION` change (scenario-def is input, not saved state).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/types.rs` (`#[cfg(test)]`) or the scenario-loader test module — deserialize-with-and-without the new fields.

### Commands

1. `cargo test -p worldwake-cli scenario`
2. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
3. `./scripts/verify.sh`
