# S107PRODIV-005: CLI wiring — AgentDef + spawn_agent for DiversificationProfile and LastProactiveExplorationTick

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — CLI/scenario layer only
**Deps**: archive/tickets/S107PRODIV-001.md

## Problem

Scenario authors need to configure `DiversificationProfile` on agents to enable proactive exploration. This requires adding the profile to `AgentDef` and wiring `spawn_agent()` to set both `DiversificationProfile` (scenario-configured, role-specific) and `LastProactiveExplorationTick` (runtime-generated, always None at spawn for agents with the profile).

## Assumption Reassessment (2026-04-17)

1. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:70`. Currently has ~31 optional profile fields. Role-specific pattern: `exploration_profile: Option<ExplorationProfileDef>` at line 115. DiversificationProfile has no EntityId fields, so no `*Def` wrapper is needed — use `Option<DiversificationProfile>` directly.
2. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:328`. Role-specific conditional pattern: `if let Some(ref profile) = agent_def.theft_disposition { txn.set_component_...(agent_id, profile.clone())?; }` (lines 431-432).
3. `LastProactiveExplorationTick` should be set to `LastProactiveExplorationTick(None)` at spawn only for agents that have `DiversificationProfile` — it's meaningless without the profile.
4. Early `cargo test --workspace --no-run` compile fallout proved the shared authored-input surface is broader than the drafted two-file scope: exhaustive `AgentDef` literals in `worldwake-cli` test helpers and handler/display scenarios also need the new optional field.

## Architecture Check

1. Follows established role-specific component wiring pattern. No `*Def` wrapper overhead since all fields are primitive types (Permille, u32, u16).
2. No backward-compatibility shims. Existing scenarios without `diversification_profile` continue to work — the field is `Option` and agents without it behave identically to today.

## Verification Layers

1. Agent with DiversificationProfile in scenario → component present after spawn → focused test
2. Agent without DiversificationProfile → component absent → focused test
3. LastProactiveExplorationTick set to None at spawn for agents with profile → focused test
4. RON scenario with explicit diversification_profile deserializes into AgentDef → focused test
5. Single-layer ticket: CLI wiring only

## What to Change

### 1. Add to AgentDef

In `crates/worldwake-cli/src/scenario/types.rs`, add field:
```rust
pub diversification_profile: Option<DiversificationProfile>,
```

### 2. Wire in spawn_agent

In `crates/worldwake-cli/src/scenario/mod.rs`, add role-specific conditional:
```rust
if let Some(ref dp) = agent_def.diversification_profile {
    txn.set_component_diversification_profile(agent_id, *dp)?;
    txn.set_component_last_proactive_exploration_tick(
        agent_id,
        LastProactiveExplorationTick(None),
    )?;
}
```

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify) — add field to AgentDef
- `crates/worldwake-cli/src/scenario/mod.rs` (modify) — add spawn_agent wiring
- `crates/worldwake-cli/src/display.rs` (modify) — update exhaustive AgentDef test literals after shared authored-input shape change
- `crates/worldwake-cli/src/handlers/actions.rs` (modify) — same-crate AgentDef literal fallout
- `crates/worldwake-cli/src/handlers/control.rs` (modify) — same-crate AgentDef literal fallout
- `crates/worldwake-cli/src/handlers/events.rs` (modify) — same-crate AgentDef literal fallout
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify) — same-crate AgentDef literal fallout
- `crates/worldwake-cli/src/handlers/tick.rs` (modify) — same-crate AgentDef literal fallout
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify) — same-crate AgentDef literal fallout

## Out of Scope

- DiversificationProfile Default impl (already in ticket 001)
- Scenario file creation (ticket 007)
- Proactive exploration logic (ticket 006)

## Acceptance Criteria

### Tests That Must Pass

1. RON scenario with `diversification_profile: Some(DiversificationProfile { ... })` deserializes correctly
2. spawn_agent sets DiversificationProfile component when present in AgentDef
3. spawn_agent sets LastProactiveExplorationTick(None) when DiversificationProfile present
4. spawn_agent does NOT set either component when diversification_profile is None
5. Existing suite: `cargo test -p worldwake-cli`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Agents without DiversificationProfile have no LastProactiveExplorationTick component
2. DiversificationProfile is role-specific — conditional `if let Some` pattern, not `unwrap_or_default`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/types.rs` — RON deserialize test with explicit `diversification_profile`
2. `crates/worldwake-cli/src/scenario/mod.rs` — spawn test with and without diversification_profile in AgentDef

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-17.

- Added `diversification_profile: Option<DiversificationProfile>` to [`crates/worldwake-cli/src/scenario/types.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/scenario/types.rs) so authored scenarios can configure proactive-diversification behavior directly without an extra `*Def` wrapper.
- Wired [`crates/worldwake-cli/src/scenario/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/scenario/mod.rs) so `spawn_agent()` sets both `DiversificationProfile` and `LastProactiveExplorationTick(None)` only when the authored agent actually has a diversification profile.
- Added focused proof for both the authored-input path and the runtime spawn path, and updated the remaining exhaustive `AgentDef` literals across `worldwake-cli` test helpers to absorb the shared CLI-schema fallout discovered by the early compile-only pass.

## Verification Result

- Passed `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_deserializes_diversification_profile -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_agent_with_diversification_profile_sets_runtime_components -- --exact`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
