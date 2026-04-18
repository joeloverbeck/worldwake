# S116DRIESCSUS-005: Scenario integration — AgentDef.drive_escalation_profile

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `AgentDef` field + `spawn_agent` application
**Deps**: archive/tickets/S116DRIESCSUS-002.md

## Problem

Spec S116 D6 requires scenarios to be able to configure `DriveEscalationProfile` per agent. Universal per CLAUDE.md §5: every agent gets the default unless the scenario overrides. Without this ticket, scenario authors cannot tune escalation parameters (e.g., tighter `start_after_ticks` for a specific contested-resource scenario), and the goldens in ticket 006 cannot override defaults when needed.

## Assumption Reassessment (2026-04-17)

1. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:70-140` holds 30+ `Option<T>` profile fields, each annotated `#[serde(default)]`. `DriveThresholds` already appears as a direct RON-typed field at line 109 — same shape applies to `DriveEscalationProfile` (no `EntityId` references).
2. `spawn_agent()` in `crates/worldwake-cli/src/scenario/mod.rs` uses the `unwrap_or_default()` pattern for universal components. Expected call site: wherever `DriveThresholds` is applied via `set_component_drive_thresholds` (grep-confirmed during implementation).
3. `DriveEscalationProfile` contains `BTreeMap<HomeostaticNeedId, DriveEscalationParams>` + `DriveEscalationParams` — all primitive + enum + scalar wrapper values, no `EntityId` references. Direct RON deserialization works without a `*Def` wrapper.
4. Existing scenario RON files under `scenarios/*.ron` omit the new field — `#[serde(default)]` keeps them valid. No mass-update is required for existing scenarios.
5. Intended layer: CLI/scenario-loading layer. No engine runtime change.

## Architecture Check

1. Universal per CLAUDE.md §5: field is `Option<DriveEscalationProfile>` in AgentDef but `unwrap_or_default()` in spawn — every agent gets a profile.
2. No `*Def` wrapper needed (spec D6 explicit) — `DriveEscalationProfile` has no cross-entity references, so the core type is directly RON-deserializable. Fewer types to maintain.
3. No shim or compatibility layer — new scenarios can set the field, old scenarios get defaults, no dual-path code.

## Verification Layers

1. RON deserialization → focused test loading a scenario with explicit `drive_escalation_profile` and asserting the component is set with the expected values.
2. Default fallback → focused test loading a scenario without the field and asserting `DriveEscalationProfile::default()` is applied.
3. Universal-profile contract → focused test asserting every spawned agent has a `DriveEscalationProfile` component (after spawn, `get_component_drive_escalation_profile` returns `Some(_)` for every agent entity).
4. Single-crate ticket (cli-only) — no cross-crate verification needed; ticket 002 already landed the component-registration surface.

## What to Change

### 1. AgentDef field

In `crates/worldwake-cli/src/scenario/types.rs:70-140`, after the existing `drive_thresholds` field at line 109, add:

```rust
#[serde(default)]
pub drive_escalation_profile: Option<DriveEscalationProfile>,
```

Update imports at the top of the file to include `DriveEscalationProfile` from `worldwake_core`.

### 2. spawn_agent application

In `crates/worldwake-cli/src/scenario/mod.rs`, inside `spawn_agent()`, at the point where `DriveThresholds` is applied (grep: `set_component_drive_thresholds` in that function), add:

```rust
world_txn.set_component_drive_escalation_profile(
    agent_id,
    def.drive_escalation_profile.clone().unwrap_or_default(),
);
```

Position: immediately after the `DriveThresholds` application for adjacency.

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — add field + import)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — add `set_component_drive_escalation_profile` call in `spawn_agent`)
- `crates/worldwake-cli/src/display.rs` and `crates/worldwake-cli/src/handlers/*.rs` test modules (modify — exhaustive `AgentDef` scenario literals gain the new optional field)

## Out of Scope

- Updating existing scenario RON files under `scenarios/*.ron` — they remain valid via `#[serde(default)]`.
- Ranking reads — ticket 004.
- needs_system reads — ticket 003.
- Bespoke `DriveEscalationProfileDef` wrapper — not needed (no `EntityId` references).

## Acceptance Criteria

### Tests That Must Pass

1. Focused scenario-loader test: loads a minimal scenario with explicit `drive_escalation_profile: Some(...)` and asserts the component is present with the expected `per_need` / `default_per_need` values.
2. Focused scenario-loader test: loads a minimal scenario without the field and asserts `DriveEscalationProfile::default()` is applied.
3. Existing scenario tests under `cargo test -p worldwake-cli scenario` still pass (all scenarios continue to load).

### Invariants

1. Every spawned agent has `DriveEscalationProfile` — whether the scenario author specified it or not.
2. Existing scenario RON files under `scenarios/*.ron` remain valid (no required field added).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/mod.rs` or `types.rs` — two focused tests for RON deserialization with and without the field.

### Commands

1. `cargo test -p worldwake-cli scenario`
2. `cargo test -p worldwake-cli`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-17.

- Added `drive_escalation_profile: Option<DriveEscalationProfile>` to `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs`, preserving omission compatibility through `#[serde(default)]`.
- Extended `spawn_agent()` in `crates/worldwake-cli/src/scenario/mod.rs` so every spawned agent gets `set_component_drive_escalation_profile(...)`, using the scenario override when present and `DriveEscalationProfile::default()` otherwise.
- Added focused parser/bootstrap proof: `types.rs` now proves explicit RON deserialization plus omission-as-`None`, and `scenario/mod.rs` now proves both default universal seeding and explicit override application.
- Absorbed the real same-crate exhaustive-literal fallout across CLI display/handler test scenario builders so `AgentDef` remains exhaustive everywhere the crate manually constructs scenarios.

## Deviations

- Reassessment confirmed this was still a CLI-owned ticket, but not strictly a two-file patch: adding the new `AgentDef` field caused expected same-crate constructor fallout in display/handler test modules that manually build exhaustive `AgentDef` literals.
- The live universal-profile contract is already enforced in `spawn_agent()` through explicit component writes, so the truthful default-fallback proof was a spawn-world assertion in `scenario/mod.rs`, not only a parser-only test.

## Verification Result

- Passed `cargo test -p worldwake-cli scenario`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
